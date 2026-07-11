use std::str::FromStr;

use miette::Result;
use tracing::trace;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Ident;
use crate::{
    ast::{
        Emit, ExprType, FromGrammar, FuncSig, GetExprType, InferExprType, NodeResultExt, ValType,
        funcs::{Param, Params, Returns},
        types::IsSubtypeOf as _,
    },
    envs::{FuncEnv, FuncSymbol, SymbolTable, VarSymbol},
    errors::{ParseError, TypeCheckError},
    locs::{Loc, Source},
};

/// An expression in the AST.
#[derive(Clone, Debug)]
pub enum Expr {
    Literal(LiteralExpr),
    Binop(BinopExpr),
    Var(VarExpr),
    Call(CallExpr),
}

impl Expr {
    /// Get the location of this expression.
    pub fn loc(&self) -> &Loc {
        match self {
            Expr::Literal(lit) => lit.loc(),
            Expr::Binop(binop) => &binop.loc,
            Expr::Var(var) => &var.loc,
            Expr::Call(call) => &call.loc,
        }
    }
}

impl FromGrammar for Expr {
    type Input<'a> = nodes::Expr<'a>;

    fn from_grammar(src: &Source, expr: nodes::Expr<'_>) -> Result<Self, ParseError> {
        match expr {
            // We unpack `nodes::Atom` without including it directly in the
            // actual AST.
            nodes::Expr::Atom(atom) => match atom {
                nodes::Atom::Ident(ident) => Ok(Expr::Var(VarExpr::from_grammar(src, ident)?)),
                nodes::Atom::Literal(lit) => {
                    Ok(Expr::Literal(LiteralExpr::from_grammar(src, lit)?))
                }
                nodes::Atom::ParenExpr(expr) => {
                    Expr::from_grammar(src, expr.expr().expect_matching())
                }
            },
            nodes::Expr::Binop(binop) => Ok(Expr::Binop(BinopExpr::from_grammar(src, binop)?)),
            nodes::Expr::Call(call) => Ok(Expr::Call(CallExpr::from_grammar(src, call)?)),
        }
    }
}

impl InferExprType for Expr {
    fn infer_expr_type(
        &mut self,
        env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        match self {
            Expr::Literal(literal_expr) => literal_expr.expr_type(),
            Expr::Binop(binop_expr) => binop_expr.infer_expr_type(env, syms),
            Expr::Var(var_expr) => var_expr.infer_expr_type(env, syms),
            Expr::Call(call_expr) => call_expr.infer_expr_type(env, syms),
        }
    }
}

impl Emit for Expr {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        match self {
            Expr::Literal(lit) => lit.emit(sink),
            Expr::Binop(binop) => binop.emit(sink),
            Expr::Var(var) => var.emit(sink),
            Expr::Call(call) => call.emit(sink),
        }
    }
}

/// A binary operator expression.
#[derive(Clone, Debug)]
pub struct BinopExpr {
    loc: Loc,
    op: Binop,
    expr1: Box<Expr>,
    expr2: Box<Expr>,

    /// Inferred operator signature. Since many of our operators are at least
    /// partially "generic" in their argument types, we generate concrete
    /// signatures on the fly during type inference. Note that this signature is
    /// must be (and is later guaranteed to be) "valid" according to the rules of the
    /// language.
    inferred_sig: Option<FuncSig>,
}

impl FromGrammar for BinopExpr {
    type Input<'a> = nodes::Binop<'a>;

    fn from_grammar(src: &Source, binop: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(binop.raw());
        let expr1 = Expr::from_grammar(src, binop.left().expect_matching())?;
        let expr2 = Expr::from_grammar(src, binop.right().expect_matching())?;
        let op_str = src.node_text(binop.op().expect_matching().raw());
        let op = match op_str {
            "+" => Binop::Add,
            "*" => Binop::Mul,
            "<" => Binop::Lt,
            ">" => Binop::Gt,
            "&&" => Binop::And,
            _ => panic!("grammar matched {op_str:?}, but it isn't implemented"),
        };
        Ok(BinopExpr {
            loc,
            op,
            expr1: Box::new(expr1),
            expr2: Box::new(expr2),
            inferred_sig: None,
        })
    }
}

impl InferExprType for BinopExpr {
    fn infer_expr_type(
        &mut self,
        env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        let ty1 = self.expr1.infer_expr_type(env, syms)?;
        let ty2 = self.expr2.infer_expr_type(env, syms)?;
        let sig = self
            .op
            .sig(&self.loc, self.expr1.loc(), &ty1, self.expr2.loc(), &ty2)?;
        let args = vec![
            (self.expr1.loc().clone(), ty1),
            (self.expr2.loc().clone(), ty2),
        ];
        let result_ty = sig.infer_call_type(&self.loc, &args)?;
        result_ty.expect_single(); // Assertion. Should already be enforced.
        self.inferred_sig = Some(sig);
        Ok(result_ty)
    }
}

impl Emit for BinopExpr {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.expr1.emit(sink)?;
        self.expr2.emit(sink)?;
        let sig = self
            .inferred_sig
            .as_ref()
            .expect("type inference should have been run");

        // Get our param type, which should _currently_ be the same for both
        // parameters.
        let params = sig.params();
        assert!(params.len() == 2);
        let param_ty = params[0].ty().clone();
        debug_assert!(param_ty.type_eq(params[1].ty()));

        // Emit an operator, using the signed variant where our parameter type
        // requires it.
        self.op.emit(param_ty.is_signed(), sink)?;

        // Perform any return-type masking required to emulate 8-bit and
        // 16-bit numeric types.
        sig.returns().expr_type()?.expect_single().emit_mask(sink)?;
        Ok(())
    }
}

/// A reference to a variable (including parameters).
#[derive(Clone, Debug)]
pub struct VarExpr {
    loc: Loc,
    ident: Ident,
    /// The variable symbol we're referencing, set during type inference.
    inferred_sym: Option<VarSymbol>,
}

impl FromGrammar for VarExpr {
    type Input<'a> = nodes::Ident<'a>;

    fn from_grammar(src: &Source, var: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(var.raw());
        let ident = Ident::from_grammar(src, var)?;
        Ok(VarExpr {
            loc,
            ident,
            inferred_sym: None,
        })
    }
}

impl InferExprType for VarExpr {
    fn infer_expr_type(
        &mut self,
        _env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        let sym = syms.get_var(&self.ident)?;
        self.inferred_sym = Some(sym.clone());
        sym.expr_type()
    }
}

impl Emit for VarExpr {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        let sym = self
            .inferred_sym
            .as_ref()
            .expect("type inference should have run");
        match sym {
            VarSymbol::Local { idx, local } => local.emit_get(*idx, sink)?,
        }
        Ok(())
    }
}

/// A function call.
#[derive(Clone, Debug)]
pub struct CallExpr {
    loc: Loc,
    func_name: Ident,
    args: Vec<Expr>,
    /// The function symbol we're referencing, set during type inference.
    inferred_sym: Option<FuncSymbol>,
}

impl FromGrammar for CallExpr {
    type Input<'a> = nodes::Call<'a>;

    fn from_grammar(src: &Source, call: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(call.raw());
        let func_name = Ident::from_grammar(src, call.func().expect_matching())?;
        let mut args = vec![];
        let mut c = call.walk();
        for arg in call.args(&mut c) {
            let arg = arg.expect_matching();
            args.push(Expr::from_grammar(src, arg)?);
        }
        Ok(CallExpr {
            loc,
            func_name,
            args,
            inferred_sym: None,
        })
    }
}

impl InferExprType for CallExpr {
    fn infer_expr_type(
        &mut self,
        env: &mut FuncEnv,
        syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        let args = self
            .args
            .iter_mut()
            .map(|arg| Ok((arg.loc().clone(), arg.infer_expr_type(env, syms)?)))
            .collect::<Result<Vec<_>>>()?;
        let sym = syms.get_func(&self.func_name)?;
        self.inferred_sym = Some(sym.clone());
        sym.func_sig().infer_call_type(&self.loc, &args)
    }
}

impl Emit for CallExpr {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        let sym = self
            .inferred_sym
            .as_ref()
            .expect("type inference should have run");

        // Emit args and call.
        for arg in &self.args {
            arg.emit(sink)?;
        }
        sink.call(sym.idx().try_as_u32()?);
        Ok(())
    }
}

/// A binary operator.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Binop {
    Add,
    Mul,
    Lt,
    Gt,
    // Logical AND.
    And,
}

impl Binop {
    /// Construct a fake identifier for this operator.
    fn ident(&self, loc: &Loc) -> Ident {
        let name = match self {
            Binop::Add => "+",
            Binop::Mul => "*",
            Binop::Lt => "<",
            Binop::Gt => ">",
            Binop::And => "&&",
        };
        Ident::new(loc.to_owned(), name.to_owned())
    }

    /// Given the parameters passed to this operator, construct a `FuncSig` with
    /// appropriate paramater and argument types. This provides a _very_ limited
    /// form of static dispatch and/or template instantiation over our numeric
    /// types where needed.
    fn sig(
        &self,
        loc: &Loc,
        loc1: &Loc,
        ty1: &ExprType,
        loc2: &Loc,
        ty2: &ExprType,
    ) -> Result<FuncSig> {
        trace!(op = ?self, ?ty1, ?ty2, "computing binop signature");
        let check_eq = || -> Result<()> {
            if ty1.type_eq(ty2) {
                Ok(())
            } else {
                Err(TypeCheckError::not_equal(loc1, ty1.to_owned(), loc2, ty2.to_owned()).into())
            }
        };
        let check_numeric = || -> Result<()> {
            if ty1.is_numeric() {
                Ok(())
            } else {
                Err(TypeCheckError::not_numeric(loc, ty1.to_owned()).into())
            }
        };

        let param1_name = Ident::new(loc1.to_owned(), "param1".to_owned());
        let param2_name = Ident::new(loc2.to_owned(), "param2".to_owned());
        let ty = ty1.expect_single().to_owned();
        let mksig = |param1_ty, param2_ty, ret_ty| {
            (
                Params::new(
                    loc.to_owned(),
                    vec![
                        Param::new(loc1.to_owned(), param1_name, param1_ty),
                        Param::new(loc2.to_owned(), param2_name, param2_ty),
                    ],
                ),
                Returns::new(loc.to_owned(), vec![ret_ty]),
            )
        };

        // Do our "type instantiation".
        let (params, returns) = match self {
            // number OP number -> number
            Binop::Add | Binop::Mul => {
                check_eq()?;
                check_numeric()?;
                mksig(ty.clone(), ty.clone(), ty)
            }
            // number OP number -> bool
            Binop::Lt | Binop::Gt => {
                check_eq()?;
                check_numeric()?;
                mksig(ty.clone(), ty, ValType::bool(loc))
            }
            // bool OP bool -> bool
            Binop::And => {
                // Rely on normal call-site checking.
                mksig(ValType::bool(loc), ValType::bool(loc), ValType::bool(loc))
            }
        };
        trace!(op = ?self, ?params, ?returns, "computed binop signature");
        Ok(FuncSig::new(
            loc.to_owned(),
            self.ident(loc),
            params,
            returns,
        ))
    }

    fn emit(&self, is_signed: bool, sink: &mut InstructionSink<'_>) -> Result<()> {
        match self {
            Binop::Add => sink.i32_add(),
            Binop::Mul => sink.i32_mul(),
            Binop::Lt if is_signed => sink.i32_lt_s(),
            Binop::Lt => sink.i32_lt_u(),
            Binop::Gt if is_signed => sink.i32_gt_s(),
            Binop::Gt => sink.i32_gt_u(),
            Binop::And => sink.i32_and(),
        };
        Ok(())
    }
}

/// A literal value.
///
/// We track location here, which affects the division of responsibility between
/// us and our child types. Our child types don't _necessarily_ implement the
/// full set of [`FromGrammar`], [`GetExprType`], and [`Emit`] themselves.
#[derive(Clone, Debug)]
pub enum LiteralExpr {
    Number { loc: Loc, n: Number },
    Bool { loc: Loc, b: bool },
}

impl LiteralExpr {
    fn loc(&self) -> &Loc {
        match self {
            LiteralExpr::Number { loc, .. } => loc,
            LiteralExpr::Bool { loc, .. } => loc,
        }
    }
}

impl FromGrammar for LiteralExpr {
    type Input<'a> = nodes::Literal<'a>;

    fn from_grammar(src: &Source, lit: Self::Input<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(lit.raw());
        match lit {
            nodes::Literal::Number(num) => Ok(LiteralExpr::Number {
                loc,
                n: Number::from_grammar(src, num)?,
            }),
            nodes::Literal::Bool(nodes::Bool::True(_)) => Ok(LiteralExpr::Bool { loc, b: true }),
            nodes::Literal::Bool(nodes::Bool::False(_)) => Ok(LiteralExpr::Bool { loc, b: false }),
        }
    }
}

impl GetExprType for LiteralExpr {
    fn expr_type(&self) -> Result<ExprType> {
        match self {
            LiteralExpr::Number { loc, n } => match n {
                Number::I8(_) => Ok(ExprType::single(ValType::i8(loc))),
                Number::U8(_) => Ok(ExprType::single(ValType::u8(loc))),
                Number::I32(_) => Ok(ExprType::single(ValType::i32(loc))),
                Number::U32(_) => Ok(ExprType::single(ValType::u32(loc))),
            },
            LiteralExpr::Bool { loc, .. } => Ok(ExprType::single(ValType::bool(loc))),
        }
    }
}

impl Emit for LiteralExpr {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        match &self {
            LiteralExpr::Number { n, .. } => n.emit(sink)?,
            LiteralExpr::Bool { b, .. } => {
                sink.i32_const(if *b { 1 } else { 0 });
            }
        }
        Ok(())
    }
}

/// A number literal.
#[derive(Clone, Debug)]
pub enum Number {
    I8(i8),
    U8(u8),
    I32(i32),
    U32(u32),
}

impl FromGrammar for Number {
    type Input<'a> = nodes::Number<'a>;

    fn from_grammar(src: &Source, n: nodes::Number<'_>) -> Result<Self, ParseError> {
        let digits_str = src.node_text(n.digits().expect_matching().raw());
        let ty = n.r#type().expect_matching();
        let loc = src.loc_for(n.raw());
        match ty {
            Some(nodes::NumberLiteralType::I8(_)) => {
                Ok(Number::I8(parse_number(loc, "i8", digits_str)?))
            }
            Some(nodes::NumberLiteralType::U8(_)) => {
                Ok(Number::U8(parse_number(loc, "u8", digits_str)?))
            }
            // Default type to i32.
            Some(nodes::NumberLiteralType::I32(_)) | None => {
                Ok(Number::I32(parse_number(loc, "i32", digits_str)?))
            }
            Some(nodes::NumberLiteralType::U32(_)) => {
                Ok(Number::U32(parse_number(loc, "u32", digits_str)?))
            }
        }
    }
}

/// Parse a number literal, returning an appropriate error if necessary.
fn parse_number<T: FromStr>(loc: Loc, type_str: &str, text: &str) -> Result<T, ParseError> {
    match text.parse::<T>() {
        Ok(val) => Ok(val),
        Err(_) => Err(ParseError::new(
            loc.span,
            format!("{} literal out of bounds: {}", type_str, text),
        )),
    }
}

impl Emit for Number {
    fn emit(&self, sink: &mut InstructionSink<'_>) -> Result<()> {
        // WASM has far fewer numeric types than we do, so we need to convert to the
        // canonical representations.
        let i32_val = match self {
            // Sign extend.
            Number::I8(v) => i32::from(*v),
            Number::U8(v) => i32::from(*v),
            Number::I32(v) => *v,
            // Reinterpret bits.
            Number::U32(v) => v.cast_signed(),
        };
        sink.i32_const(i32_val);
        Ok(())
    }
}

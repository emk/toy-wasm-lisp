use std::str::FromStr;

use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Ident;
use crate::{
    ast::{
        ExprType, FromGrammar, GetExprType, InferExprType, NodeResultExt, ValType,
        types::IsSubtypeOf as _,
    },
    envs::{LocalEnv, SymbolTable, VarSymbol},
    errors::{ParseError, TypeCheckError},
    locs::{Loc, Source},
};

#[derive(Clone, Debug)]
pub struct Expr {
    loc: Loc,
    variant: ExprVariant,
}

impl Expr {
    pub fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.variant.emit(env, sink)
    }
}

impl FromGrammar for Expr {
    type Input<'a> = nodes::Expr<'a>;

    fn from_grammar(src: &Source, expr: nodes::Expr<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(expr.raw());
        let variant = match expr {
            nodes::Expr::Atom(atom) => ExprVariant::from_grammar_atom(src, atom)?,
            nodes::Expr::Binop(binop) => ExprVariant::from_grammar_binop(src, binop)?,
            nodes::Expr::Call(call) => ExprVariant::from_grammar_call(src, call)?,
        };
        Ok(Self { loc, variant })
    }
}

impl InferExprType for Expr {
    fn infer_expr_type(&mut self, symbol_table: &SymbolTable<'_>) -> Result<ExprType> {
        match &mut self.variant {
            ExprVariant::Literal(lit) => match lit {
                Literal::Number(n) => match n {
                    Number::I8(_) => Ok(ExprType::single(ValType::i8(&self.loc))),
                    Number::U8(_) => Ok(ExprType::single(ValType::u8(&self.loc))),
                    Number::I32(_) => Ok(ExprType::single(ValType::i32(&self.loc))),
                    Number::U32(_) => Ok(ExprType::single(ValType::u32(&self.loc))),
                },
                Literal::Bool(_) => Ok(ExprType::single(ValType::bool(&self.loc))),
            },
            ExprVariant::Binop {
                ty,
                op: _,
                expr1,
                expr2,
            } => {
                let ty1 = expr1.infer_expr_type(symbol_table)?;
                let ty2 = expr2.infer_expr_type(symbol_table)?;
                if !ty1.is_numeric() || !ty2.is_numeric() {
                    return Err(TypeCheckError::not_numeric(&self.loc, ty1).into());
                }
                if !ty1.type_eq(&ty2) {
                    return Err(TypeCheckError::not_equal(&self.loc, ty1, ty2).into());
                }
                *ty = Some(ty1.expect_single().clone());
                Ok(ty1)
            }
            ExprVariant::Var(ident) => {
                let sym = symbol_table.get_var(ident)?;
                sym.expr_type()
            }
            ExprVariant::Call { func_name, args } => {
                let (_idx, sig) = symbol_table.get_func(func_name)?;
                if args.len() != sig.params().len() {
                    return Err(TypeCheckError::wrong_number_of_args(
                        &self.loc,
                        sig.params().len(),
                        args.len(),
                    )
                    .into());
                }
                for (arg, param) in args.iter_mut().zip(sig.params().iter()) {
                    let arg_ty = arg.infer_expr_type(symbol_table)?;
                    let param_ty = param.ty();
                    arg_ty.expecting(&arg.loc, &ExprType::single(param_ty.to_owned()))?;
                }
                sig.returns().expr_type()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExprVariant {
    Literal(Literal),
    Binop {
        /// Inferred operator type, for both arguments and return value. This
        /// needs to be resolved to a [`ValType`] by this point, because we
        /// don't support using multi-value expressions as arguments to binops.
        ty: Option<ValType>,
        op: Binop,
        expr1: Box<Expr>,
        expr2: Box<Expr>,
    },
    Var(Ident),
    Call {
        func_name: Ident,
        args: Vec<Expr>,
    },
}

impl ExprVariant {
    fn from_grammar_atom(src: &Source, atom: nodes::Atom<'_>) -> Result<Self, ParseError> {
        match atom {
            nodes::Atom::Ident(ident) => Ok(ExprVariant::Var(Ident::from_grammar(src, ident)?)),
            nodes::Atom::Literal(lit) => Ok(ExprVariant::Literal(Literal::from_grammar(src, lit)?)),
            nodes::Atom::ParenExpr(expr) => {
                Ok(Expr::from_grammar(src, expr.expr().expect_matching())?.variant)
            }
        }
    }

    fn from_grammar_binop(src: &Source, binop: nodes::Binop<'_>) -> Result<Self, ParseError> {
        let expr1 = Expr::from_grammar(src, binop.left().expect_matching())?;
        let expr2 = Expr::from_grammar(src, binop.right().expect_matching())?;
        let op_str = src.node_text(binop.op().expect_matching().raw());
        let op = match op_str {
            "+" => Binop::Add,
            "*" => Binop::Mul,
            _ => panic!("grammar matched {op_str:?}, but it isn't implemented"),
        };
        Ok(ExprVariant::Binop {
            ty: None,
            op,
            expr1: Box::new(expr1),
            expr2: Box::new(expr2),
        })
    }

    fn from_grammar_call(src: &Source, call: nodes::Call<'_>) -> Result<Self, ParseError> {
        let func_name = Ident::from_grammar(src, call.func().expect_matching())?;
        let mut args = vec![];
        let mut c = call.walk();
        for arg in call.args(&mut c) {
            let arg = arg.expect_matching();
            args.push(Expr::from_grammar(src, arg)?);
        }
        Ok(ExprVariant::Call { func_name, args })
    }

    fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        match &self {
            ExprVariant::Literal(lit) => {
                lit.emit(env, sink)?;
            }
            ExprVariant::Binop {
                ty,
                op,
                expr1,
                expr2,
            } => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                op.emit(env, sink)?;
                let ty = ty.as_ref().expect("type inference should have been run");
                ty.emit_mask(sink)?;
            }
            ExprVariant::Var(name) => match env.symbol_table().get_var(name)? {
                VarSymbol::Local { idx, local } => local.emit_get(*idx, sink)?,
            },
            ExprVariant::Call { func_name, args } => {
                let (idx, _func) = env.symbol_table().get_func(func_name)?;

                // Emit args and call.
                for arg in args {
                    arg.emit(env, sink)?;
                }
                sink.call(idx.try_as_u32()?);
            }
        }
        Ok(())
    }
}

/// A binary operator.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Binop {
    Add,
    Mul,
}

impl Binop {
    fn emit(&self, _env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        match self {
            Binop::Add => sink.i32_add(),
            Binop::Mul => sink.i32_mul(),
        };
        Ok(())
    }
}

/// A literal value.
#[derive(Clone, Debug)]
pub enum Literal {
    Number(Number),
    Bool(bool),
}

impl Literal {
    fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        match &self {
            Literal::Number(n) => n.emit(env, sink)?,
            Literal::Bool(b) => {
                sink.i32_const(if *b { 1 } else { 0 });
            }
        }
        Ok(())
    }
}

impl FromGrammar for Literal {
    type Input<'a> = nodes::Literal<'a>;

    fn from_grammar(src: &Source, lit: Self::Input<'_>) -> Result<Self, ParseError> {
        match lit {
            nodes::Literal::Number(num) => Ok(Literal::Number(Number::from_grammar(src, num)?)),
            nodes::Literal::Bool(nodes::Bool::True(_)) => Ok(Literal::Bool(true)),
            nodes::Literal::Bool(nodes::Bool::False(_)) => Ok(Literal::Bool(false)),
        }
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

impl Number {
    fn emit(&self, _env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
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

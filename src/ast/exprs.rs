use std::str::FromStr;

use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Ident;
use crate::{
    ast::{ExprType, FromGrammar, InferExprType, NodeResultExt, ValType, types::IsSubtypeOf as _},
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
    fn infer_expr_type(&self, symbol_table: &SymbolTable<'_>) -> Result<ExprType> {
        match &self.variant {
            ExprVariant::Number(n) => match n {
                Number::I8(_) | Number::U8(_) => {
                    unimplemented!("change type system for i8/u8 on stack")
                }
                Number::I32(_) => Ok(ExprType::single(ValType::i32(&self.loc))),
                Number::U32(_) => Ok(ExprType::single(ValType::u32(&self.loc))),
            },
            ExprVariant::Add { expr1, expr2 } | ExprVariant::Mul { expr1, expr2 } => {
                let ty1 = expr1.infer_expr_type(symbol_table)?;
                let ty2 = expr2.infer_expr_type(symbol_table)?;
                if !ty1.is_numeric() || !ty2.is_numeric() {
                    return Err(TypeCheckError::not_numeric(&self.loc, ty1).into());
                }
                if !ty1.type_eq(&ty2) {
                    return Err(TypeCheckError::not_equal(&self.loc, ty1, ty2).into());
                }
                Ok(ty1)
            }
            ExprVariant::Var(ident) => {
                let sym = symbol_table.get_var(ident)?;
                sym.infer_expr_type(symbol_table)
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
                for (arg, param) in args.iter().zip(sig.params().iter()) {
                    let arg_ty = arg.infer_expr_type(symbol_table)?;
                    let param_ty = param.ty();
                    arg_ty.expecting(&arg.loc, &ExprType::single(param_ty.to_owned()))?;
                }
                sig.returns().infer_expr_type(symbol_table)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExprVariant {
    Number(Number),
    Add { expr1: Box<Expr>, expr2: Box<Expr> },
    Mul { expr1: Box<Expr>, expr2: Box<Expr> },
    Var(Ident),
    Call { func_name: Ident, args: Vec<Expr> },
}

impl ExprVariant {
    fn from_grammar_atom(src: &Source, atom: nodes::Atom<'_>) -> Result<Self, ParseError> {
        match atom {
            nodes::Atom::Ident(ident) => Ok(ExprVariant::Var(Ident::from_grammar(src, ident)?)),
            nodes::Atom::Number(num) => Ok(ExprVariant::Number(Number::from_grammar(src, num)?)),
            nodes::Atom::ParenExpr(expr) => {
                Ok(Expr::from_grammar(src, expr.expr().expect_matching())?.variant)
            }
        }
    }

    fn from_grammar_binop(src: &Source, binop: nodes::Binop<'_>) -> Result<Self, ParseError> {
        let expr1 = Expr::from_grammar(src, binop.left().expect_matching())?;
        let expr2 = Expr::from_grammar(src, binop.right().expect_matching())?;
        let op = src.node_text(binop.op().expect_matching().raw());
        match op {
            "+" => Ok(ExprVariant::Add {
                expr1: Box::new(expr1),
                expr2: Box::new(expr2),
            }),
            "*" => Ok(ExprVariant::Mul {
                expr1: Box::new(expr1),
                expr2: Box::new(expr2),
            }),
            _ => panic!("grammar matched {op:?}, but it isn't implemented"),
        }
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
            ExprVariant::Number(n) => {
                n.emit(env, sink)?;
            }
            ExprVariant::Add { expr1, expr2 } => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                sink.i32_add();
            }
            ExprVariant::Mul { expr1, expr2 } => {
                expr1.emit(env, sink)?;
                expr2.emit(env, sink)?;
                sink.i32_mul();
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

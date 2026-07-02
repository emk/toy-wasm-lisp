use miette::Result;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Ident;
use crate::{
    ast::{ExprType, InferExprType, NodeResultExt, ValType, types::IsSubtypeOf as _},
    envs::{LocalEnv, SymbolTable, VarSymbol},
    errors::TypeCheckError,
    locs::{Loc, Source},
};

#[derive(Clone, Debug)]
pub struct Expr {
    loc: Loc,
    variant: ExprVariant,
}

impl Expr {
    pub fn from_grammar(src: &Source, expr: nodes::Expr<'_>) -> Self {
        let loc = src.loc_for(expr.raw());
        let variant = match expr {
            nodes::Expr::Atom(atom) => ExprVariant::from_grammar_atom(src, atom),
            nodes::Expr::Binop(binop) => ExprVariant::from_grammar_binop(src, binop),
            nodes::Expr::Call(call) => ExprVariant::from_grammar_call(src, call),
        };
        Self { loc, variant }
    }

    pub fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.variant.emit(env, sink)
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
    fn from_grammar_atom(src: &Source, atom: nodes::Atom<'_>) -> Self {
        match atom {
            nodes::Atom::Ident(ident) => ExprVariant::Var(Ident::from_grammar(src, ident)),
            nodes::Atom::Number(num) => ExprVariant::Number(Number::from_grammar(src, num)),
            nodes::Atom::ParenExpr(expr) => {
                Expr::from_grammar(src, expr.expr().expect_matching()).variant
            }
        }
    }

    fn from_grammar_binop(src: &Source, binop: nodes::Binop<'_>) -> Self {
        let expr1 = Expr::from_grammar(src, binop.left().expect_matching());
        let expr2 = Expr::from_grammar(src, binop.right().expect_matching());
        let op = src.node_text(binop.op().expect_matching().raw());
        match op {
            "+" => ExprVariant::Add {
                expr1: Box::new(expr1),
                expr2: Box::new(expr2),
            },
            "*" => ExprVariant::Mul {
                expr1: Box::new(expr1),
                expr2: Box::new(expr2),
            },
            _ => panic!("grammar matched {op:?}, but it isn't implemented"),
        }
    }

    fn from_grammar_call(src: &Source, call: nodes::Call<'_>) -> Self {
        let func_name = Ident::from_grammar(src, call.func().expect_matching());
        let mut args = vec![];
        let mut c = call.walk();
        for arg in call.args(&mut c) {
            let arg = arg.expect_matching();
            args.push(Expr::from_grammar(src, arg));
        }
        ExprVariant::Call { func_name, args }
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
    fn from_grammar(src: &Source, n: nodes::Number<'_>) -> Self {
        let digits_str = src.node_text(n.digits().expect_matching().raw());
        let ty = n.r#type().expect_matching();
        // TODO: Thread error-handling through.
        match ty {
            Some(nodes::NumberLiteralType::I8(_)) => Number::I8(digits_str.parse().unwrap()),
            Some(nodes::NumberLiteralType::U8(_)) => Number::U8(digits_str.parse().unwrap()),
            Some(nodes::NumberLiteralType::I32(_)) | None => {
                Number::I32(digits_str.parse().unwrap())
            }
            Some(nodes::NumberLiteralType::U32(_)) => Number::U32(digits_str.parse().unwrap()),
        }
    }

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

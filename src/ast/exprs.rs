use miette::{Result, miette};
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
            ExprVariant::Number(_) => Ok(ExprType::single(ValType::i32(&self.loc))),
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
                sig.returns().infer_expr_type(symbol_table)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExprVariant {
    Number(i32),
    Add { expr1: Box<Expr>, expr2: Box<Expr> },
    Mul { expr1: Box<Expr>, expr2: Box<Expr> },
    Var(Ident),
    Call { func_name: Ident, args: Vec<Expr> },
}

impl ExprVariant {
    fn from_grammar_atom(src: &Source, atom: nodes::Atom<'_>) -> Self {
        match atom {
            nodes::Atom::Ident(ident) => ExprVariant::Var(Ident::from_grammar(src, ident)),
            nodes::Atom::Number(num) => ExprVariant::Number(
                src.node_text(num.raw())
                    .parse()
                    // TODO: Huh, do we really need to thread error-handling through
                    // the entire grammar conversion now? 🤦
                    .expect("integer out of bounds"),
            ),
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
            ExprVariant::Number(i) => {
                sink.i32_const(*i);
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
                let (idx, func) = env.symbol_table().get_func(func_name)?;
                let func_type = func.wasm_func_type()?;

                // TODO: Actually set up type checking.
                if func_type.params().len() != args.len() {
                    return Err(miette!(
                        "expected {} arguments, got {}",
                        func_type.params().len(),
                        args.len()
                    ));
                }
                if func_type.results().len() != 1 {
                    return Err(miette!(
                        "expected 1 result, got {}",
                        func_type.results().len()
                    ));
                }

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

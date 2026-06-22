use std::sync::Arc;

use miette::{NamedSource, Result, miette};
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::InstructionSink;

use super::Ident;
use crate::{
    ast::{NodeResultExt, node_source},
    envs::{LocalEnv, Symbol, SymbolCategory},
    errors::SymbolTableError,
    locs::Loc,
};

#[derive(Clone, Debug)]
pub struct Expr {
    #[expect(dead_code)]
    loc: Loc,
    variant: ExprVariant,
}

impl Expr {
    pub fn from_grammar(src: Arc<NamedSource<String>>, expr: nodes::Expr<'_>) -> Self {
        let loc = Loc::new(src.clone(), expr.raw());
        let variant = match expr {
            nodes::Expr::Atom(atom) => ExprVariant::from_grammar_atom(src.clone(), atom),
            nodes::Expr::Binop(binop) => ExprVariant::from_grammar_binop(src.clone(), binop),
            nodes::Expr::Call(call) => ExprVariant::from_grammar_call(src.clone(), call),
        };
        Self { loc, variant }
    }

    pub fn emit(&self, env: &LocalEnv<'_>, sink: &mut InstructionSink<'_>) -> Result<()> {
        self.variant.emit(env, sink)
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
    fn from_grammar_atom(src: Arc<NamedSource<String>>, atom: nodes::Atom<'_>) -> Self {
        match atom {
            nodes::Atom::Ident(ident) => ExprVariant::Var(Ident::from_grammar(src, ident)),
            nodes::Atom::Number(num) => ExprVariant::Number(
                node_source(&src, num.raw())
                    .parse()
                    // TODO: Huh, do we really need to thread error-handling through
                    // the entire grammar conversion now? 🤦
                    .expect("integer out of bounds"),
            ),
            nodes::Atom::Parenexpr(expr) => {
                Expr::from_grammar(src, expr.expr().expect_matching()).variant
            }
        }
    }

    fn from_grammar_binop(src: Arc<NamedSource<String>>, binop: nodes::Binop<'_>) -> Self {
        let expr1 = Expr::from_grammar(src.clone(), binop.left().expect_matching());
        let expr2 = Expr::from_grammar(src.clone(), binop.right().expect_matching());
        let op = node_source(&src, binop.op().expect_matching().raw());
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

    fn from_grammar_call(src: Arc<NamedSource<String>>, call: nodes::Call<'_>) -> Self {
        let func_name = Ident::from_grammar(src.clone(), call.func().expect_matching());
        let mut args = vec![];
        let mut c = call.walk();
        for arg in call.args(&mut c) {
            let arg = arg.expect_matching();
            args.push(Expr::from_grammar(src.clone(), arg));
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
            ExprVariant::Var(name) => match env.symbol_table().get(name)? {
                Symbol::Func { .. } => {
                    return Err(SymbolTableError::wrong_symbol_category(
                        name.to_owned(),
                        SymbolCategory::Var,
                        SymbolCategory::Func,
                    )
                    .into());
                }
                Symbol::Local { idx, local } => local.emit_get(*idx, sink)?,
            },
            ExprVariant::Call { func_name, args } => {
                let (idx, func) = env.symbol_table().get_func(func_name)?;
                let func_type = func.func_type()?;

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

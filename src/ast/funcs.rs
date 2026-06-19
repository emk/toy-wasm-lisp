use std::sync::Arc;

use miette::{NamedSource, Result};
use rust_sitter::Spanned;
use wasm_encoder::{FuncType, Function, ValType};

use super::Block;
use crate::{envs::ModuleEnv, locs::Loc, parser::grammar};

#[derive(Clone, Debug)]
pub struct Func {
    #[expect(dead_code)]
    loc: Loc,
    should_export: bool,
    name: grammar::Ident,
    params: Params,
    returns: Returns,
    body: Block,
}

impl Func {
    pub fn from_grammar(src: Arc<NamedSource<String>>, block: &Spanned<grammar::Func>) -> Self {
        let loc = Loc::new(src.clone(), block);
        Self {
            loc,
            should_export: block.export.is_some(),
            name: block.name.clone(),
            params: Params::from_grammar(&block.params),
            returns: Returns::from_grammar(&block.returns),
            body: Block::from_grammar(src, &block.body),
        }
    }

    pub fn should_export(&self) -> bool {
        self.should_export
    }

    pub fn func_type(&self) -> Result<FuncType> {
        Ok(FuncType::new(self.params.types()?, self.returns.types()?))
    }

    pub fn emit_decl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        mod_env.insert_function(self.name.clone(), self.clone())
    }

    pub fn emit_impl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        let locals = vec![];
        let mut f = Function::new(locals);
        let mut sink = f.instructions();
        self.body.emit(mod_env, &mut sink)?;
        sink.end();
        mod_env.insert_code(&f);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Params {
    params: Vec<Spanned<grammar::Param>>,
}

impl Params {
    pub fn from_grammar(params: &grammar::Params) -> Self {
        Self {
            params: params.params.clone(),
        }
    }

    fn types(&self) -> Result<Vec<ValType>> {
        self.params
            .iter()
            .map(|p| p.ty.value.val_type())
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Clone, Debug)]
pub struct Returns {
    tys: Vec<Spanned<grammar::Type>>,
}

impl Returns {
    pub fn from_grammar(returns: &Option<grammar::Returns>) -> Self {
        let tys = match returns {
            None => vec![],
            Some(grammar::Returns::Single { ty, .. }) => vec![ty.clone()],
            Some(grammar::Returns::Multiple { tys, .. }) => tys.clone(),
        };
        Self { tys }
    }

    fn types(&self) -> Result<Vec<ValType>> {
        self.tys
            .iter()
            .map(|ty| ty.value.val_type())
            .collect::<Result<Vec<_>>>()
    }
}

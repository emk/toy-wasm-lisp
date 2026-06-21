use std::sync::Arc;

use miette::{NamedSource, Result};
use rust_sitter::Spanned;
use wasm_encoder::{FuncType, Function, ValType};

use super::{Block, Ident, Type};
use crate::{
    ast::Local,
    envs::{DeclTable, LocalEnv, ModuleEnv},
    locs::Loc,
    parser::grammar,
};

#[derive(Clone, Debug)]
pub struct Func {
    #[expect(dead_code)]
    loc: Loc,
    should_export: bool,
    name: Ident,
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
            name: Ident::from_grammar(src.clone(), &block.name),
            params: Params::from_grammar(src.clone(), &block.params),
            returns: Returns::from_grammar(src.clone(), &block.returns),
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
        // Set up a LocalEnv, and seed it with our parameters.
        let mut decls = DeclTable::new();
        let mut local_env = LocalEnv::new(&mut decls, mod_env.symbol_table());
        self.params.declare(&mut local_env)?;

        let locals = vec![];
        let mut f = Function::new(locals);
        let mut sink = f.instructions();
        self.body.emit(&local_env, &mut sink)?;
        sink.end();
        mod_env.insert_code(&f);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Params {
    #[expect(dead_code)]
    loc: Loc,
    params: Vec<Param>,
}

impl Params {
    pub fn from_grammar(src: Arc<NamedSource<String>>, params: &Spanned<grammar::Params>) -> Self {
        let loc = Loc::new(src.clone(), params);
        Self {
            loc,
            params: params
                .params
                .iter()
                .map(|p| Param::from_grammar(src.clone(), p))
                .collect::<Vec<_>>(),
        }
    }

    fn types(&self) -> Result<Vec<ValType>> {
        self.params
            .iter()
            .map(|p| p.ty.val_type())
            .collect::<Result<Vec<_>>>()
    }

    fn declare(&self, local_env: &mut LocalEnv) -> Result<()> {
        for param in &self.params {
            param.declare(local_env)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Param {
    #[expect(dead_code)]
    loc: Loc,
    name: Ident,
    ty: Type,
}

impl Param {
    fn from_grammar(src: Arc<NamedSource<String>>, param: &Spanned<grammar::Param>) -> Self {
        let loc = Loc::new(src.clone(), param);
        Self {
            loc,
            name: Ident::from_grammar(src.clone(), &param.name),
            ty: Type::from_grammar(src.clone(), &param.ty),
        }
    }

    fn declare(&self, local_env: &mut LocalEnv) -> Result<()> {
        let local = Local::new(self.name.clone(), self.ty.clone());
        local_env.insert_local(self.name.clone(), local)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Returns {
    #[expect(dead_code)]
    loc: Loc,
    tys: Vec<Type>,
}

impl Returns {
    pub fn from_grammar(
        src: Arc<NamedSource<String>>,
        returns: &Spanned<Option<grammar::Returns>>,
    ) -> Self {
        let loc = Loc::new(src.clone(), returns);
        let tys = match &returns.value {
            None => vec![],
            Some(grammar::Returns::Single { ty, .. }) => vec![Type::from_grammar(src, ty)],
            Some(grammar::Returns::Multiple { tys, .. }) => tys
                .iter()
                .map(|ty| Type::from_grammar(src.clone(), ty))
                .collect::<Vec<_>>(),
        };
        Self { loc, tys }
    }

    fn types(&self) -> Result<Vec<ValType>> {
        self.tys
            .iter()
            .map(|ty| ty.val_type())
            .collect::<Result<Vec<_>>>()
    }
}

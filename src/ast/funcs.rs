use std::ops::Index;

use miette::Result;
use tracing::trace;
use tree_sitter_wasl_types::nodes;
use type_sitter::Node as _;
use wasm_encoder::{FuncType, Function, ValType as WasmValType};

use super::{
    Block, ExprType, GetExprType, Ident, InferExprType, LocalSymbol, NodeResultExt, ToWasmType,
    ValType,
};
use crate::{
    ast::{Emit as _, FromGrammar},
    envs::{FuncEnv, ModuleEnv, SymbolTable},
    errors::{ParseError, TypeCheckError},
    locs::{Loc, Source},
};

#[derive(Clone, Debug)]
pub struct Func {
    #[expect(dead_code)]
    loc: Loc,
    should_export: bool,
    sig: FuncSig,
    body: Block,
}

impl Func {
    pub fn should_export(&self) -> bool {
        self.should_export
    }

    pub fn sig(&self) -> &FuncSig {
        &self.sig
    }

    pub fn emit_decl(&self, mod_env: &mut ModuleEnv) -> Result<()> {
        mod_env.insert_function(self.sig.name.clone(), self.clone())
    }

    pub fn emit_impl(&mut self, mod_env: &mut ModuleEnv) -> Result<()> {
        // Set up a LocalEnv, and seed it with our parameters.
        let mut func_env = FuncEnv::new();
        let mut syms = mod_env.symbol_table().child();
        func_env.declare_params(&self.sig, &mut syms)?;

        // TODO: Redesign type inference.
        let body_ty = self.body.infer_expr_type(&mut func_env, &mut syms)?;
        body_ty.expecting(&self.body.loc, &self.sig.returns.expr_type()?)?;

        let mut locals = vec![];
        for (_idx, local) in func_env.locals() {
            locals.push((1, local.expr_type()?.expect_single().to_wasm_type()));
        }
        trace!(f = %self.sig.name(), ?locals, "Declaring WASM locals");
        let mut f = Function::new(locals);
        let mut sink = f.instructions();
        self.body.emit(&mut sink)?;
        sink.end();
        mod_env.insert_code(&f);
        Ok(())
    }
}

impl FromGrammar for Func {
    type Input<'a> = nodes::Func<'a>;

    fn from_grammar(src: &Source, func: nodes::Func<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(func.raw());
        Ok(Self {
            loc,
            should_export: func.export().is_some(),
            sig: FuncSig::from_grammar(src, func.sig().expect_matching())?,
            body: Block::from_grammar(src, func.body().expect_matching())?,
        })
    }
}

/// A function signature. This is shared between imported functions and locally
/// defined functions.
#[derive(Clone, Debug)]
pub struct FuncSig {
    #[expect(dead_code)]
    loc: Loc,
    name: Ident,
    params: Params,
    returns: Returns,
}

impl FuncSig {
    /// Manually construct a [`FuncSig`]. Used for things like operators.
    pub fn new(loc: Loc, name: Ident, params: Params, returns: Returns) -> Self {
        Self {
            loc,
            name,
            params,
            returns,
        }
    }

    pub fn name(&self) -> &Ident {
        &self.name
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn returns(&self) -> &Returns {
        &self.returns
    }

    /// Infer the type of a call.
    pub fn infer_call_type(&self, loc: &Loc, args: &[(Loc, ExprType)]) -> Result<ExprType> {
        trace!(sig = ?self, ?args, "inferring call type");
        if args.len() != self.params.len() {
            return Err(
                TypeCheckError::wrong_number_of_args(loc, self.params.len(), args.len()).into(),
            );
        }
        for ((arg_loc, arg_ty), param) in args.iter().zip(self.params.iter()) {
            let param_ty = param.ty();
            arg_ty.expecting(arg_loc, &ExprType::single(param_ty.to_owned()))?;
        }
        self.returns().expr_type()
    }

    pub fn wasm_func_type(&self) -> Result<FuncType> {
        Ok(FuncType::new(
            self.params.wasm_types()?,
            self.returns.wasm_types()?,
        ))
    }
}

impl FromGrammar for FuncSig {
    type Input<'a> = nodes::FuncSig<'a>;

    fn from_grammar(src: &Source, sig: nodes::FuncSig<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(sig.raw());
        let params = sig.params().expect_matching();
        let returns = sig.returns().expect_matching();
        let returns_loc = match returns {
            Some(returns) => src.loc_for(returns.raw()),
            None => src.loc_after(params.raw()),
        };
        Ok(Self {
            loc,
            name: Ident::from_grammar(src, sig.name().expect_matching())?,
            params: Params::from_grammar(src, params)?,
            returns: Returns::from_grammar(src, (returns_loc, returns))?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Params {
    #[expect(dead_code)]
    loc: Loc,
    params: Vec<Param>,
}

impl Params {
    /// Create a new parameter list. Used internally for things like operators.
    pub fn new(loc: Loc, params: Vec<Param>) -> Self {
        Self { loc, params }
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.params.iter()
    }

    fn wasm_types(&self) -> Result<Vec<WasmValType>> {
        Ok(self
            .params
            .iter()
            .map(|p| p.ty.to_wasm_type())
            .collect::<Vec<_>>())
    }

    pub fn declare(&self, local_env: &mut FuncEnv, syms: &mut SymbolTable) -> Result<()> {
        for param in &self.params {
            param.declare(local_env, syms)?;
        }
        Ok(())
    }
}

/// Index a param.
impl Index<usize> for Params {
    type Output = Param;

    fn index(&self, index: usize) -> &Self::Output {
        &self.params[index]
    }
}

impl FromGrammar for Params {
    type Input<'a> = nodes::Params<'a>;

    fn from_grammar(src: &Source, params: nodes::Params<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(params.raw());
        let mut cursor = params.walk();
        Ok(Self {
            loc,
            params: params
                .params(&mut cursor)
                .map(|p| Param::from_grammar(src, p.expect_matching()))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Param {
    #[expect(dead_code)]
    loc: Loc,
    name: Ident,
    ty: ValType,
}

impl Param {
    /// Create a new parameter list. Used internally for things like operators.
    pub fn new(loc: Loc, name: Ident, ty: ValType) -> Self {
        Param { loc, name, ty }
    }

    fn declare(&self, local_env: &mut FuncEnv, syms: &mut SymbolTable) -> Result<()> {
        let local = LocalSymbol::new(self.name.clone(), self.ty.clone());
        local_env.insert_local(self.name.clone(), local, syms)?;
        Ok(())
    }

    pub fn ty(&self) -> &ValType {
        &self.ty
    }
}

impl FromGrammar for Param {
    type Input<'a> = nodes::Param<'a>;

    fn from_grammar(src: &Source, param: nodes::Param<'_>) -> Result<Self, ParseError> {
        let loc = src.loc_for(param.raw());
        Ok(Self {
            loc,
            name: Ident::from_grammar(src, param.name().expect_matching())?,
            ty: ValType::from_grammar(src, param.r#type().expect_matching())?,
        })
    }
}

impl InferExprType for Param {
    fn infer_expr_type(
        &mut self,
        _env: &mut FuncEnv,
        _syms: &mut SymbolTable<'_>,
    ) -> Result<ExprType> {
        Ok(ExprType::single(self.ty.clone()))
    }
}

#[derive(Clone, Debug)]
pub struct Returns {
    #[expect(dead_code)]
    loc: Loc,
    tys: Vec<ValType>,
}

impl Returns {
    /// Create a new returns list. Used internally for things like operators.
    pub fn new(loc: Loc, tys: Vec<ValType>) -> Self {
        Self { loc, tys }
    }

    fn wasm_types(&self) -> Result<Vec<WasmValType>> {
        Ok(self
            .tys
            .iter()
            .map(|ty| ty.to_wasm_type())
            .collect::<Vec<_>>())
    }
}

impl FromGrammar for Returns {
    type Input<'a> = (Loc, Option<nodes::Returns<'a>>);

    fn from_grammar(
        src: &Source,
        (loc, returns): (Loc, Option<nodes::Returns<'_>>),
    ) -> Result<Self, ParseError> {
        let mut tys = vec![];
        match returns {
            None => {}
            Some(returns) => {
                if let Some(ty) = returns.single() {
                    tys.push(ValType::from_grammar(src, ty.expect_matching())?);
                } else {
                    let mut c = returns.walk();
                    for ty in returns.multiples(&mut c) {
                        tys.push(ValType::from_grammar(src, ty.expect_matching())?);
                    }
                }
            }
        };
        Ok(Self { loc, tys })
    }
}

impl GetExprType for Returns {
    fn expr_type(&self) -> Result<ExprType> {
        Ok(ExprType::multiple(self.tys.iter().cloned()))
    }
}

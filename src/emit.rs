//! Emit an AST.

use miette::Result;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Module, TypeSection,
};

use crate::parser::grammar::Func;

pub fn emit_func(func: &Func) -> Result<Vec<u8>> {
    // TODO: Split this up some.
    let mut types = TypeSection::new();
    types
        .ty()
        .function(func.param_types()?, func.return_types()?);
    let f_type_id = 0;

    let mut functions = FunctionSection::new();
    functions.function(f_type_id);
    let f_func_id = 0;

    let mut exports = ExportSection::new();
    if func.is_exported() {
        exports.export(&func.name.text, ExportKind::Func, f_func_id);
    }

    let mut codes = CodeSection::new();
    let locals = vec![];
    let mut f = Function::new(locals);
    let mut sink = f.instructions();
    func.body.emit(&mut sink)?;
    sink.end();
    codes.function(&f);

    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&exports);
    module.section(&codes);

    Ok(module.finish())
}

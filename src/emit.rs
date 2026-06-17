//! Emit an AST.

use miette::Result;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, InstructionSink, Module,
    TypeSection, ValType,
};

use crate::parser::grammar::{Expr, Func};

pub fn emit(ast: &Func) -> Result<Vec<u8>> {
    let mut types = TypeSection::new();
    types.ty().function(vec![], vec![ValType::I32]);
    let f_type_id = 0;

    let mut functions = FunctionSection::new();
    functions.function(f_type_id);
    let f_func_id = 0;

    let mut exports = ExportSection::new();
    exports.export(&ast.name, ExportKind::Func, f_func_id);

    let mut codes = CodeSection::new();
    let locals = vec![];
    let mut f = Function::new(locals);
    let mut sink = f.instructions();
    emit_expr(&mut sink, &ast.expr)?;
    sink.end();
    codes.function(&f);

    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&exports);
    module.section(&codes);

    Ok(module.finish())
}

fn emit_expr(sink: &mut InstructionSink<'_>, expr: &Expr) -> Result<()> {
    match expr {
        Expr::Number(i) => {
            sink.i32_const(*i);
        }
        Expr::Add(expr1, _, expr2) => {
            emit_expr(sink, expr1)?;
            emit_expr(sink, expr2)?;
            sink.i32_add();
        }
        Expr::Mul(expr1, _, expr2) => {
            emit_expr(sink, expr1)?;
            emit_expr(sink, expr2)?;
            sink.i32_mul();
        }
    }
    Ok(())
}

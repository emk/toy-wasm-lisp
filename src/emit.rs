//! Emit an AST.

use miette::Result;
use rust_sitter::Spanned;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, InstructionSink, Module,
    TypeSection, ValType,
};

use crate::parser::grammar::{Expr, Func, Params, Returns, Type};

pub fn emit_func(func: &Func) -> Result<Vec<u8>> {
    let mut types = TypeSection::new();
    types.ty().function(
        get_param_types(&func.params)?,
        get_return_types(&func.returns)?,
    );
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
    emit_expr(&mut sink, &func.body.expr)?;
    sink.end();
    codes.function(&f);

    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&exports);
    module.section(&codes);

    Ok(module.finish())
}

fn get_param_types(params: &Params) -> Result<Vec<ValType>> {
    params
        .tys
        .iter()
        .map(|p| get_val_type(&p.ty))
        .collect::<Result<Vec<_>>>()
}

fn get_return_types(returns: &Option<Returns>) -> Result<Vec<ValType>> {
    match returns {
        None => Ok(vec![]),
        Some(Returns::Single { ty, .. }) => Ok(vec![get_val_type(ty)?]),
        Some(Returns::Multiple { tys, .. }) => {
            tys.iter().map(get_val_type).collect::<Result<Vec<_>>>()
        }
    }
}

fn get_val_type(ty: &Spanned<Type>) -> Result<ValType> {
    match &ty.value {
        Type::I32 => Ok(ValType::I32),
    }
}

fn emit_expr(sink: &mut InstructionSink<'_>, expr: &Spanned<Expr>) -> Result<()> {
    match &expr.value {
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

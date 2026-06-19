use std::{fs, path::PathBuf};

use clap::Parser;
use miette::{Context, IntoDiagnostic, Result, miette};
use tracing::debug;
use tracing_subscriber::{EnvFilter, fmt};
use wasmtime::{Engine, Linker, Module, Store};

use crate::ast::parse;

mod ast;
mod envs;
mod errors;

#[derive(Debug, Parser)]
enum Opt {
    Run { path: PathBuf },
}

fn main() -> Result<()> {
    init_tracing();

    let opt = Opt::parse();
    debug!(?opt, "Options");
    let Opt::Run { path } = opt;

    let src = fs::read_to_string(&path)
        .into_diagnostic()
        .with_context(|| format!("Failed to read input file: {}", path.display()))?;

    let mod_ast = parse(&path.to_string_lossy(), &src)?;
    debug!(?mod_ast, "Parsed");
    let wasm = mod_ast.emit()?;
    let wat = wasmprinter::print_bytes(&wasm).map_err(|e| miette!("{e}"))?;
    debug!(%wat, "Compiled");

    let engine = Engine::default();
    let linker = Linker::new(&engine);
    let module = Module::new(&engine, &wasm).map_err(|e| miette!("{e}"))?;
    let mut store: Store<()> = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| miette!("{e}"))?;
    let f = instance
        .get_typed_func::<(), (i32,)>(&mut store, "f")
        .map_err(|e| miette!("{e}"))?;
    let (value,) = f.call(&mut store, ()).map_err(|e| miette!("{e}"))?;
    println!("Output: {value}");

    Ok(())
}

/// Initialize the `tracing` subscriber. Reads `RUST_LOG` if set; otherwise
/// defaults to `info` for this crate and `warn` for everything else. Writes
/// to stderr so it does not pollute sandboxed child-process stdout.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("toy_wasm_lisp=info,warn"));
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

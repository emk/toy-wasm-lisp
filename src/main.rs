use miette::{Result, miette};
use tracing::debug;
use tracing_subscriber::{EnvFilter, fmt};
use wasmtime::{Engine, Linker, Module, Store};

use crate::{emit::emit, parser::parse};

mod emit;
mod errors;
mod parser;

fn main() -> Result<()> {
    init_tracing();
    let ast = parse("<input>", "1 ++ 2 * 3")?;
    debug!(?ast, "Parsed");
    let wasm = emit(&ast)?;
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

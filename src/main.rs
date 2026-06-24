use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use miette::{Context, IntoDiagnostic, Result, miette};
use tracing::{debug, trace};
use tracing_subscriber::{EnvFilter, field::MakeExt as _, fmt};
use wasmtime::{Caller, Engine, Instance, Linker, Module, Store};

use crate::parser::parse;

mod ast;
mod envs;
mod errors;
mod locs;
mod parser;

#[derive(Debug, Parser)]
enum Opt {
    Run { path: PathBuf },
}

fn main() -> Result<()> {
    init_tracing();

    let opt = Opt::parse();
    debug!(?opt, "Options");
    let Opt::Run { path } = opt;

    let (mut store, instance) = compile_and_instantiate(&path)?;
    let start = instance
        .get_typed_func::<(), (i32,)>(&mut store, "_start")
        .map_err(|e| miette!("{e}"))?;
    let (output,) = start.call(&mut store, ()).map_err(|e| miette!("{e}"))?;
    debug!(output, "Output");
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
        .map_fmt_fields(|f| f.debug_alt())
        .init();
}

/// Compile source code to WASM, and instantiate it.
fn compile_and_instantiate(path: &Path) -> Result<(Store<()>, Instance)> {
    let src = fs::read_to_string(path)
        .into_diagnostic()
        .with_context(|| format!("Failed to read input file: {}", path.display()))?;

    let parsed = parse(&path.to_string_lossy(), &src)?;
    trace!(?parsed, "Parsed");
    let wasm = parsed.emit()?;
    let wat = wasmprinter::print_bytes(&wasm).map_err(|e| miette!("{e}"))?;
    debug!(%wat, "Compiled");

    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    if cfg!(test) {
        // If we're running tests, install a `wasl_test` API.
        linker
            .func_wrap("wasl_test", "the_answer", |_caller: Caller<'_, ()>| 42)
            .map_err(|e| miette!("{e}"))?;
    }
    let module = Module::new(&engine, &wasm).map_err(|e| miette!("{e}"))?;
    let mut store: Store<()> = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| miette!("{e}"))?;
    Ok((store, instance))
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    fn init_test_tracing() {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("toy_wasm_lisp=info,warn"));
        let _ = fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .map_fmt_fields(|f| f.debug_alt())
            .try_init();
    }

    fn call_f(store: &mut Store<()>, instance: &Instance) -> Result<i32> {
        let f = instance
            .get_typed_func::<(), (i32,)>(&mut *store, "f")
            .map_err(|e| miette!("{e}"))?;
        let (value,) = f.call(store, ()).map_err(|e| miette!("{e}"))?;
        debug!(value, "Output");
        Ok(value)
    }

    #[test]
    fn compile_and_run_test_programs() -> Result<()> {
        init_test_tracing();

        let re = Regex::new(r"// EXPECT: f\(\) == (\d+)").expect("invalid regex");
        for entry in glob::glob("tests/fixtures/**/*.wasl")
            .into_diagnostic()
            .context("invalid glob pattern")?
        {
            let path = entry.into_diagnostic().context("glob error")?;
            let src = fs::read_to_string(&path)
                .into_diagnostic()
                .with_context(|| format!("reading {}", path.display()))?;

            let Some(caps) = re.captures(&src) else {
                return Err(miette!(
                    "no recognizable 'EXPECT' comment in {}",
                    path.display()
                ));
            };
            let expected: i32 = caps[1]
                .parse()
                .into_diagnostic()
                .with_context(|| miette!("invalid expected value in {}", path.display()))?;

            let (mut store, instance) = compile_and_instantiate(&path)?;
            let value = call_f(&mut store, &instance)?;

            assert_eq!(value, expected, "{}", path.display());
        }
        Ok(())
    }
}

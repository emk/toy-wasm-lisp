use miette::Result;
use tracing_subscriber::{EnvFilter, fmt};

use crate::parser::parse;

mod errors;
mod parser;

fn main() -> Result<()> {
    init_tracing();
    let ast = parse("<input>", "1 + 2 * 3")?;
    println!("{:#?}", ast);
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

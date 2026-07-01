# Contributing

This is a personal project to learn about WASM GC. It's not really in a state to be used or accept contributions, but if you're interested, please get in touch.

See README.md for instructions on building and running the Lisp-based experiment and the more complete Rust-based experiment.

## Source Layout

- `crates/`: Supporting crates.
  - `tree-sitter-wasl/`: `tree-sitter` grammar for the experimental WASM-based "system" language.
  - `tree-sitter-wasl-types/`: `type-sitter` wrapper for the grammar, providing stronger typing in Rust. This is a separate create to avoid slow recompilations as the grammar grows.
- `docs/`: Assorted project documentation and notes.
- `runtime/`: Two different runtime implementations that might be used by a toy Lisp.
  - `wasl/`: A runtime using our so-called "WebAssembly Systems Language".
  - `watm/`: A runtime using Lisp-like macros layered over raw WAT syntax. This gets a little messy when the concrete syntax for things differs too much between the languages.
- `src/`: Rust source code for compiling and running "WebAssembly Systems Language".
  - `ast/`: Abstract syntax tree types. These are created from the `type-sitter` wrappers using `*::from_grammar` methods, type-checked, and compiled to WASM.
  - `envs/`: We need a surprising amount of namespace/environment machinery to make the compiler work.
    - `module_env.rs`: WASM module-level environment.
    - `symbol_table.rs`: Our main symbol table.
  - `locs.rs`: Source files and locations in them.
  - `parser.rs`: Interface to `tree-sitter` and `type-sitter` parsing machinery.
- `texts/fixtures/`: Source files that can be compiled, including what output or errors we expect.
- `watm-assembler.lisp`: The "assembler" for our WAT extensions with Lisp macros.

## Rust style

We're using `miette` and `tracing`. TODO: Elaborate on style details.

## AI Policy

Since this is a project designed to learn things and try out new ideas, AI use is extremely minimal. Specifically, it's restricted to side tasks like asking questions about WASM and troubleshooting tree-sitter weirdness. (The documentation for both WASM and tree-sitter is pretty dire.) AI is not used for writing code. AI-written PRs will not be accepted.

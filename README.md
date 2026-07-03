# WORK IN PROGRESS: Toy Lisp for WASM

This does not do anything yet. I am experimenting with various ill-conceived macroassembler ideas as a way of forcing myself to learn low-level WASM GC details.

## WATM: WAT + Lisp-like macros 

This is an entertainingly bad idea. To build and run:

```sh
sbcl --load watm-assembler.lisp --quit && \
    wasm-tools parse runtime/watm/runtime.wat -o runtime/watm/runtime.wasm && \
    wasmtime -W gc runtime/watm/runtime.wasm
```

This is largely abandoned and kept for reference and history.

## WASL: WebAssembly Systems Language

An ongoing experiment at staying really close to the "metal" (lower-level than C). To run:

```sh
env RUST_LOG=toy_wasm_lisp=debug,warn cargo run -- \
    run runtime/wasl/runtime.wasl
```

### Force rebuilding the grammar (fish shell)

You can build everything using `just`, if it's installed:

```sh
just build
```

Then reload `rust-analyzer` so it notices the change.

### Testing

As usual:

```sh
cargo test
```

This includes quite a few test programs, both ones expected to return a value, and ones expected to fail with an error.

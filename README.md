# WORK IN PROGRESS: Toy Lisp for WASM

This does not do anything yet. I am experimenting with various ill-conceived macroassembler ideas as a way of forcing myself to learn low-level WASM GC details.

Building:

```sh
sbcl --load wat-assembler.lisp --quit &&
  wasm-tools parse runtime/runtime.wat -o runtime/runtime.wasm &&
  wasmtime -W gc runtime/runtime.wasm
```

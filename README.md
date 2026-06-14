# WORK IN PROGRESS: Toy Lisp for WASM

This does not do anything yet.

Building:

```sh
sbcl --load wat-assembler.lisp --quit &&
  wasm-tools parse runtime/runtime.wat -o runtime/runtime.wasm
```

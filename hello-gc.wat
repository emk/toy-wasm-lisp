;; Decompiled from Rust and extensively modified by hand. Contains
;; various experiments.
(module $hello-gc.wasm
  ;; System interface.
  (type $fd_write_type (func (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (type $fd_write_type)))
  (type $proc_exit_type (func (param i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (type $proc_exit_type)))

  ;; Memory layout.
  (memory $memory 17)
  (export "memory" (memory $memory))

  (global $__stack_pointer (mut i32)
    i32.const 1048576)
  (global $_data_end i32
    i32.const 1048590)
  (export "__data_end" (global $_data_end))
  (global $_heap_base i32
    i32.const 1048592)
  (export "__heap_base" (global $_heap_base))

  ;; Core Lisp types.
  ;;
  ;; We put these into a (rec) mostly because it supposedly adds some
  ;; limited measure of nominal typing for things like downcasting.
  ;; As I understand it, WASM uses structural equivalence, but it uses
  ;; it across the entire (rec)?
  (rec
    (type $str (array i8))
    (type $symbol
      (struct
        (field $name (ref $str))
        (field $value (mut (ref null any)))
        ;; TODO: Probably want a closure type here.
        (field $function (mut (ref null any)))))
    (type $cons
      (struct
        (field $car (mut (ref any)))
        (field $cdr (mut (ref any))))))

  (global $nil_str (ref $str)
    (array.new_fixed $str 3
      (i32.const 0x6E) ;; n
      (i32.const 0x69) ;; i
      (i32.const 0x6c))) ;; l

  (global $nil (ref $symbol)
    (struct.new $symbol
      (global.get $nil_str)
      (ref.null any)
      (ref.null any)))

  (type $prim_cons_type (func (param (ref any) (ref any)) (result (ref any))))
  (func $prim_mkcons (type $prim_cons_type)
    (struct.new $cons
      (local.get 0)
      (local.get 1)))

  (type $_start_type (func))
  (export "_start" (func $_start))
  (func $_start (type $_start_type)
    (local i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 0
    global.set $__stack_pointer
    local.get 0
    i32.const 14
    i32.store offset=8
    local.get 0
    i32.const 1048576
    i32.store offset=4
    local.get 0
    i32.const 0
    i32.store offset=12
    i32.const 1
    local.get 0
    i32.const 4
    i32.add
    i32.const 1
    local.get 0
    i32.const 12
    i32.add
    call $fd_write
    drop

    global.get $nil
    global.get $nil
    call $prim_mkcons
    drop

    i32.const 0
    call $proc_exit
    unreachable)
  (data $.rodata
    (i32.const 1048576) "Hello, world!\0a"))

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

  ;;(type $prim_cons_type (func  ))

  (func $prim_mkcons ;;(type $prim_cons_type)
    (param $car (ref any))
    (param $cdr (ref any))
    (result (ref any))
    (struct.new $cons
      (local.get $car)
      (local.get $cdr)))

  (func (export "_start")
    ;; Frame pointer?
    (local $__fp i32)

    ;; Reserve 16 bytes for stack frame.
    ;; $__stack_pointer+12: iovs[0].buf
    ;; $__stack_pointer+8: iovs[0].buf_len
    ;; $__stack_pointer+4: iovs[0].buf
    ;; $__stack_pointer+0: ???
    (i32.sub (global.get $__stack_pointer) (i32.const 16))
    (local.tee $__fp)
    (global.set $__stack_pointer)

    ;; Build iovs.
    (i32.store offset=8 (local.get $__fp) (i32.const 14)) ;; iovs[0].buf_len
    (i32.store offset=4 (local.get $__fp) (i32.const 1048576)) ;; iovs[0].buf

    ;; Build nwritten (output).
    (i32.store offset=12 (local.get $__fp) (i32.const 0))

    ;; Param fd.
    (i32.const 1)

    ;; Param iovs.
    (i32.add (local.get $__fp) (i32.const 4))

    ;; Param iovs_len.
    (i32.const 1)

    ;; Param &nwritten.
    (i32.add (local.get $__fp) (i32.const 12))
    (call $fd_write)
    (drop)

    (call $prim_mkcons (global.get $nil) (global.get $nil))
    (drop)

    (call $proc_exit (i32.const 0))
    unreachable)
  (data $.rodata
    (i32.const 1048576) "Hello, world!\0a"))

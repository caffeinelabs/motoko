(module
  (type $t_bi (func (param i32) (param i32) (result i32)))
  (type $t_i  (func (param i32) (result i32)))

  (import "rts" "quux" (func $quux (type $t_bi)))

  (table (;0;) i64 1 1 funcref)
  (memory (;0;) i64 2)
  (global $heap_base i64 (i64.const 65536))
  (export "__heap_base" (global $heap_base))
  (export "foo" (func $foo))

  ;; Zero-forwarder: body matches `zero_forwarder_target`'s syntactic
  ;; pattern (Const 0; LocalGet 1; Call quux).  Its own closure
  ;; (local 0) is discarded, and a null closure is synthesised for
  ;; the callee.
  (func $bar (type $t_bi)
    i32.const 0
    local.get 1
    call $quux)

  ;; Caller: the closure-arg for $bar is the result of a Block whose
  ;; body has a BrIf and whose then/else paths both produce `i32.const 0`.
  ;;
  ;; Phase 2 of ConstTrack evicted the Block's result slot whenever a
  ;; BrIf was seen — so the `0` at depth 1 would be lost and
  ;; `collect_rewrites` would not fire.  Phase 3 takes the intersection
  ;; of fall-through and every `Br*` branch-target state, so the shared
  ;; `i32.const 0` survives and the call site is rewritten to $quux.
  (func $foo (type $t_i)
    (block (result i32)
      i32.const 0
      local.get 0
      br_if 0
      drop
      i32.const 0
    )
    local.get 0
    call $bar))

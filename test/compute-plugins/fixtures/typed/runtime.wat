(module
  (import "env" "memory" (memory i64 0))
  (import "env" "__memory_base" (global i64))
  (import "env" "__table_base" (global i64))
  (import "env" "compute_plugin_alloc" (func $alloc (param i64 i64) (result i64)))
  (func (export "transform") (param $value i64) (param $output i64)
    (local $scratch i64)
    ;; Exercise managed scratch allocation and alignment normalization.
    (local.set $scratch (call $alloc (i64.const 1) (i64.const 1)))
    (i64.store8 (local.get $scratch) (local.get $value))
    (i64.store8 (local.get $output) (i64.load8_u (local.get $scratch)))))

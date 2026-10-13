(module
  (import "env" "memory" (memory i64 0))
  (import "env" "__memory_base" (global i64))
  (import "env" "__table_base" (global i64))
  (func (export "transform") (param $input i64) (param $size i64) (param $output i64)
    (memory.copy (local.get $output) (local.get $input) (local.get $size))))

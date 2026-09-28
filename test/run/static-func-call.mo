(func() { assert true; })()

// CHECK: func $init
// CHECK-NOT: call_indirect
// CHECK: call $@anon-func-

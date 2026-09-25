func go () {
  let foobar1 = func foobar1() { assert true; };
  foobar1();
};
go();

let foobar2 = func foobar2() { assert true; };
foobar2();

// CHECK-LABEL: func $init
// CHECK-NOT: call_indirect
// CHECK: call $foobar2

// CHECK-LABEL: func $go
// CHECK-NOT: call_indirect
// CHECK: call $foobar1

//MOC-FLAG -A=M0194
let FOO = 1;
func go (x:Nat) {
  func rec(x:Nat) {
    ignore(FOO);
  };
  rec(x);
};
go(1000);

// CHECK: func $go
// CHECK-NOT: call_indirect
// CHECK: call $rec

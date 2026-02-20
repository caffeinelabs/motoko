//MOC-FLAG -A=M0194
func bar() {
func foo() = ();
()
};
()

// CHECK-NOT: (func $foo
// CHECK-NOT: (func $bar

//SKIP run
//SKIP run-low
//SKIP run-ir

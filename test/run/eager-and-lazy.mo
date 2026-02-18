//MOC-FLAG -A=M0194
// x is mutable. Will it be heap allocated or local?
var x = 1;
// used eager and lazy
let (y, f) = (x, func () { assert( x == 2 ) });
// change
x := 2;
// observe that the capture value also changes
f();

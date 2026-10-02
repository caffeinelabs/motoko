//MOC-FLAG -A=M0194 -W=M0145
// a failing pattern match that can be compiled to a trap
let none : ?Nat = null;
let ?b = none;

//SKIP run-low
//SKIP run-ir

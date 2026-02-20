//MOC-FLAG -A=M0194
// check types and terms have separate namespaces
module X = { public type x = Int; public let x = 1; };
type x = X.x;
let x = X.x;
/* assert (x == 1); */

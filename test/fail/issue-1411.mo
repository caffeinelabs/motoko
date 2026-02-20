//MOC-FLAG -A=M0194
import M "lib/M";

let Z1 : M.F = M.F();

type T1 = Z1.T;  //ok
let x1 = Z1.x;  //ok

let (Z2 : M.F) = M.F();

type T2 = Z2.T;  //ok
let x2 = Z2.x;  //ok

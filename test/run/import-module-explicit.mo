//MOC-FLAG -A=M0194
import { cons; nil = empty } = "lib/ListM";

//type stack = List<Int>;
let s = cons(1, empty());
let u = cons<Int>(2, empty<Int>());

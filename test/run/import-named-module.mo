//MOC-FLAG -A=M0194
import L = "lib/NamedListM";
type stack = L.List<Int>;
let is = L.cons<Int>(1, L.nil<Int>());
let ts = L.map<Int,Text>(func i { debug_show(i) }, is);
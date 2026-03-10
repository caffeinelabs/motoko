//MOC-FLAG -A=M0194
// checking
let (#a a1) or (#b a1) : { #a : Nat; #b : Char } = #a 5;
let (#a (a2 : Nat)) or (#b a2) : { #a : Nat; #b : Int } = #b 5; // don't warn
let (#a (a3 : Nat)) or (#b a3) = #b (5 : Int); // don't warn
let (#a (a4 : Char)) or (#b a4) : { #a : Char; #b : Int } = #b 5;
let (#a (a5 : Char)) or (#b a5) = #b (5 : Int);

// inference
// Note: inference for or-patterns of sum type is problematic,
//       https://github.com/dfinity/motoko/issues/3993
//func foo(#a (a6 : Nat) or #b (a6 : Bool)) {};

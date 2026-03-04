//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// syntax
// trailing semis optional
let b = { b = 6 };
let ba = { b with a = 8 };
let ba_semi = { b with a = 8; };

let bac = { b with a = 8; c = true };
let bac_semi = { b with a = 8; c = true; };

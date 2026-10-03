let a = ^3; // type error, operator is not defined for operand type
let b = ^-5; // type error, operator is not defined for operand type
let c : Nat = +3; // literal type mismatch
let d : Nat = -3; // literal type mismatch

switch (1) { case (^1) { ignore "hmmm" }; case _ { ignore "unexpected" } }; // type error, operator cannot consume expected type
switch (1) { case (+1) { ignore "unexpected" }; case _ { ignore "hmmm" } }; // literal type mismatch
switch (1) { case (-1) { ignore "hmmm" }; case _ { ignore "unexpected" } }; // literal type mismatch

switch (-1) { case (^1) { ignore "hmmm" }; case _ { ignore "unexpected" } }; // type error, operator cannot consume expected type
ignore (switch (-1) { case (+1) "hmmm"; case _ "as expected" });
ignore (switch (-1) { case (-1) "as expected"; case _ "hmmm" });

ignore (switch (^1 : Int8) { case (^1) "as expected"; case _ "hmmm" });
ignore (switch (+1 : Int8) { case (+1) "as expected"; case _ "hmmm" });
ignore (switch (-1 : Int8) { case (-1) "as expected"; case _ "hmmm" });

// when testing the switch expression in the REPL, I either
//  * expect the result being "as expected", or
//  * a type error (as described in the comment)
//
// test steps wrapped in ignore should type-check

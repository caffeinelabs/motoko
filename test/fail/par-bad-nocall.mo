// A parenthetical note must directly precede a call: the parser has no call node to attach it to otherwise (M0210).
func f(x : Nat) : { y : Nat } = { y = x };
let a = (with cycles = 1) 42;
let b = (with cycles = 1) f(1).y;

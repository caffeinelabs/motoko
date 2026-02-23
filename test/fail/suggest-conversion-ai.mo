//MOC-FLAG --package conv conv --ai-errors
// should report conversions from all available libs, even when not imported
import Nat8 = "mo:conv/Nat8"
func f(n : Nat) {};
let n8 : Nat8 = 0;
let n16 : Nat16 = 0;
f(n8);
f(n16);



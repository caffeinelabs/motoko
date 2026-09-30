// Candidates are merged only when they are provably the same value, never
// because their types agree.

import Impl1 "implicit-nested-reexport/Impl1";
import Both "implicit-nested-reexport/Both";

func render(n : Nat, show : (implicit : Nat -> Text)) : Text { show(n) };

// Its type is the type of Impl1, its value is Impl2
let Impl = Both.all[1];
ignore render(5); // Error, `Impl.show` and `Impl1.show` differ

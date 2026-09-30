// A library that re-exports a module must not make its implicits ambiguous
// with a direct import of that module: all paths below reach one `show`.

import Show "implicit-nested-reexport/Show";
import Lib "implicit-nested-reexport/lib";
import { Show = Picked } "implicit-nested-reexport/lib";
import Wrapper "implicit-nested-reexport/Wrapper";

let Local = Lib.Inner.Show;
let Annotated : module { show : Nat -> Text } = Wrapper.Show;

func render(n : Nat, show : (implicit : Nat -> Text)) : Text { show(n) };

assert render(5) == "5";

// Keep every import used, independent of which path the implicit takes
assert Show.show(1) == Lib.Show.show(1);
assert Picked.show(1) == Wrapper.show(1);
assert Local.show(1) == Annotated.show(1);

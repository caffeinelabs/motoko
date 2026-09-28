import Prim "mo:⛔";

type WithMut = { var x : Int; y : Text };

func reject (o : WithMut) =
  switch o {
    case { y = "good" } { Prim.debugPrint "good" }
    //case { x = -42 } { Prim.debugPrint "bad" }; // backend: Invalid_argument("unop")
    //case { x = (42 : Int) } { Prim.debugPrint "bad" }; // tc: cannot consume expected
    case { x } { Prim.debugPrintInt x }
  };

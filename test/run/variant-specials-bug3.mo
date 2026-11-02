//MOC-FLAG -W=M0145
import Prim "mo:⛔";

func specials(two : { #admin : Text; #user : Nat }) : Text =
  switch two {
    case (#admin t) t;
    case (#admin u) u; // note: duplicate label, so the tag test must be kept
  };

Prim.debugPrint(specials(#admin "ok"));
Prim.debugPrint(specials(#user 42)) // must trap, not read a Nat back as Text

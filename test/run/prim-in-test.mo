// This test just checks that we can use
// the `prim` syntax in our tests

//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";
assert(Prim.rts_version() == ((prim "rts_version" : () -> Text) ()));

import { principalOfActor; actorOfPrincipal } = "mo:⛔";

// set up
let p = principalOfActor(actor "aaaaa-aa");

// verify
assert debug_show p == "aaaaa-aa";

// now roundtrip
assert p == principalOfActor(actorOfPrincipal<actor { getInt : () -> async Int }> p);

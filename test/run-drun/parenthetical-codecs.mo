import Prim "mo:⛔";

// `public (with encoder; decoder)` replaces Candid on the wire, per method and per direction
actor {
  func utf8(t : Text) : Blob = Prim.encodeUtf8 t;

  func text(b : Blob) : Text = switch (Prim.decodeUtf8 b) {
    case (?t) t;
    case null Prim.trap "not UTF-8"
  };

  func decimal(b : Blob) : Nat {
    var n = 0;
    for (c in text(b).chars()) {
      let d = Prim.charToNat32 c;
      if (d < 48 or d > 57) Prim.trap "not a decimal";
      n := n * 10 + Prim.nat32ToNat(d - 48)
    };
    n
  };

  // Candid in, raw UTF-8 out
  public (with encoder = utf8) func greet(name : Text) : async Text = "hello " # name;

  // raw decimal in, Candid out
  public (with decoder = decimal) func double(n : Nat) : async Nat = 2 * n;

  // raw UTF-8 in and out
  public (with decoder = text; encoder = utf8) func shout(t : Text) : async Text = t # "!";
};

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress greet "DIDL\x00\x01\x71\x05world"
//CALL ingress double "21"
//CALL ingress shout "hey"

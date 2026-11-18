import Prim "mo:⛔";

// `(with encoder; decoder) public func` replaces Candid on the wire, per method and per direction
actor {
  func utf8(t : Text) : Blob = Prim.encodeUtf8(t);

  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  func decimal(b : Blob) : Nat {
    var n = 0;
    for c in text(b).chars() {
      let d = Prim.charToNat32(c);
      if d < 48 or d > 57 { Prim.trap("not a decimal") };
      n := n * 10 + Prim.nat32ToNat(d - 48)
    };
    n
  };

  // Candid in, raw UTF-8 out
  (with encoder = utf8)
  public func greet(name : Text) : async Text = "hello " # name;

  // raw decimal in, Candid out
  (with decoder = decimal)
  public func double(n : Nat) : async Nat = 2 * n;

  // raw UTF-8 in and out
  (with decoder = text; encoder = utf8)
  public func shout(t : Text) : async Text = t # "!";

  // queries: raw UTF-8 in and out; Candid in, raw UTF-8 out; composite
  (with decoder = text; encoder = utf8)
  public query func peek(t : Text) : async Text { t # "?" };
  (with encoder = utf8)
  public query func hello() : async Text { "hi" };
  (with decoder = text; encoder = utf8)
  public composite query func echo(t : Text) : async Text { t # t };

  // raw decimal in, oneway (no reply to encode)
  (with decoder = decimal)
  public func triple(n : Nat) : () { Prim.debugPrint(debug_show(3 * n)) }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress greet "DIDL\x00\x01\x71\x05world"
//CALL ingress double "21"
//CALL ingress shout "hey"
//CALL ingress triple "14"
//CALL query peek "hey"
//CALL ingress peek "hey"
//CALL query hello "DIDL\x00\x00"
//CALL query echo "ab"

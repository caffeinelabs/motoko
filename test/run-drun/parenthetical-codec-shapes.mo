import Prim "mo:⛔";
import Mix "parenthetical-codec-shapes/Mix";

// Codecs of any shape are ordinary code: module members, lambdas using
// `Prim.trap`, `debug_show` and `==`, non-constant values, shared function
// types in the signature, several arguments/results, mixins.
actor {
  include Mix("mix");

  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  // a module member as encoder
  (with encoder = Prim.encodeUtf8)
  public func hello() : async Text { "hello" };

  // lambdas using `Prim.trap`, `debug_show` and structural equality
  (with decoder = func (b : Blob) : Nat {
    if b == ("" : Blob) { Prim.trap("empty") };
    b.size()
  }; encoder = func (n : Nat) : Blob = Prim.encodeUtf8(debug_show(n)))
  public func size(n : Nat) : async Nat { n };

  // a codec mentioning an actor-local type
  type Count = Nat;
  (with encoder = func (n : Count) : Blob = Prim.encodeUtf8(debug_show(n + 1)))
  public func count() : async Count { 41 };

  // a non-constant codec value
  func mkDecoder(k : Nat) : Blob -> Nat = func (b : Blob) : Nat { k * b.size() };
  transient let scaled = mkDecoder(10);
  (with decoder = scaled)
  public func scale(n : Nat) : async Nat { n };

  // a shared function type in the signature
  (with encoder = func (_ : shared () -> async ()) : Blob = "fn")
  public func callback() : async (shared () -> async ()) { ping };
  public func ping() : async () {};

  // several arguments, several results, unit result
  (with decoder = func (b : Blob) : (Text, Nat) = (text(b), b.size()))
  public func pair(t : Text, n : Nat) : async Nat { t.size() + n };
  (with encoder = func ((t, n) : (Text, Nat)) : Blob = Prim.encodeUtf8(t # debug_show(n)))
  public func two() : async (Text, Nat) { ("seven", 7) };
  // a unit result: the encoder takes no argument (a fixed, crafted reply)
  func crafted() : Blob = "ok";
  (with encoder = crafted)
  public func unit() : async () {};
  (with encoder = func () : Blob = "fixed")
  public func unitLambda() : async () {};
  transient let craftedCodecs = { encoder = crafted };
  (craftedCodecs with)
  public func unitBase() : async () {}
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress hello "DIDL\x00\x00"
//CALL ingress size "abc"
//CALL ingress scale "abcd"
//CALL ingress callback "DIDL\x00\x00"
//CALL ingress pair "abc"
//CALL ingress two "DIDL\x00\x00"
//CALL ingress unit "DIDL\x00\x00"
//CALL ingress unitLambda "DIDL\x00\x00"
//CALL ingress unitBase "DIDL\x00\x00"
//CALL ingress tagged "DIDL\x00\x00"
//CALL ingress count "DIDL\x00\x00"
//CALL ingress size ""

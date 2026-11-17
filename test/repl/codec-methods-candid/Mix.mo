import Prim "mo:⛔";

mixin() {
  func mixUtf8(t : Text) : Blob = Prim.encodeUtf8(t);
  public (with encoder = mixUtf8) func mixed() : async Text { "m" };
  public func plainMixed() : async Nat { 0 }
}

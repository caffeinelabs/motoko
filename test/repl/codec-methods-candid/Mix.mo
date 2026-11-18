import Prim "mo:⛔";

mixin() {
  func mixUtf8(t : Text) : Blob = Prim.encodeUtf8(t);
  (with encoder = mixUtf8)
  public func mixed() : async Text { "m" };
  public func plainMixed() : async Nat { 0 }
}

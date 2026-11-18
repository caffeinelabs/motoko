import Prim "mo:⛔";

mixin(tag : Text) {
  // a codec closing over a mixin parameter
  (with encoder = func (t : Text) : Blob = Prim.encodeUtf8(tag # t))
  public func tagged() : async Text { "!" }
}

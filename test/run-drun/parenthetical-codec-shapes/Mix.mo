import Prim "mo:⛔";

mixin(tag : Text) {
  // a codec closing over a mixin parameter
  public (with encoder = func (t : Text) : Blob = Prim.encodeUtf8(tag # t)) func tagged() : async Text { "!" }
}

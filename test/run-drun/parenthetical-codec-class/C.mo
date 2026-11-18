import Prim "mo:⛔";

// codecs in an actor class, closing over the class parameter
actor class C(prefix : Text) {
  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  (with decoder = text; encoder = func (t : Text) : Blob = Prim.encodeUtf8(prefix # t))
  public func greet(name : Text) : async Text { name }
}

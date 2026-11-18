import Prim "mo:⛔";
import Mix "Mix";

actor {
  include Mix();

  func utf8(t : Text) : Blob = Prim.encodeUtf8(t);
  func text(b : Blob) : Text = switch Prim.decodeUtf8(b) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };

  public func plain(t : Text) : async Text { t };
  (with encoder = utf8)
  public func encoded(t : Text) : async Text { t };
  (with decoder = text)
  public func decoded(t : Text) : async Text { t };
  (with decoder = text; encoder = utf8)
  public query func both(t : Text) : async Text { t };

  transient let codecs = { decoder = text };
  (codecs with)
  public func based(t : Text) : async Text { t }
}

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
  public (with encoder = utf8) func encoded(t : Text) : async Text { t };
  public (with decoder = text) func decoded(t : Text) : async Text { t };
  public (with decoder = text; encoder = utf8) query func both(t : Text) : async Text { t }
}

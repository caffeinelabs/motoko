import Prim "mo:⛔";

// A base record shares codecs between methods: `(codecs with) public func …`;
// explicit fields override the base.
actor {
  func text(b : Blob) : Text = switch (Prim.decodeUtf8(b)) {
    case ?t { t };
    case null { Prim.trap("not UTF-8") }
  };
  func utf8(t : Text) : Blob = Prim.encodeUtf8(t);

  transient let codecs = { decoder = text; encoder = utf8 };

  (codecs with)
  public func shout(t : Text) : async Text { t # "!" };
  (codecs with)
  public query func peek(t : Text) : async Text { t # "?" };
  (codecs with encoder = func (t : Text) : Blob = utf8("<" # t # ">"))
  public func wrap(t : Text) : async Text { t };
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress shout "hey"
//CALL query peek "hey"
//CALL ingress wrap "hey"

// A base in a method's parenthetical supplies codecs; they are checked like
// explicit ones: wrong type (M0214), an encoder on a oneway (M0280),
// unrecognised attributes warn (M0212), and a base with no attributes is
// as redundant as `(with)` (M0211).
actor {
  func count(_ : Blob) : Nat = 0;
  func blob(_ : Text) : Blob = "";

  transient let wrong = { decoder = count };
  (wrong with)
  public func m(t : Text) : async () {};

  transient let encoding = { encoder = blob };
  (encoding with)
  public func ping() : () {};

  transient let extra = { decoder = count; colour = "red" };
  (extra with)
  public func n(_ : Nat) : async () {};

  transient let empty = {};
  (empty with)
  public func o() : async () {}
}

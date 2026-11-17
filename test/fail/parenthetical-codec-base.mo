// A base in a method's parenthetical supplies codecs; they are checked like
// explicit ones: wrong type (M0214), an encoder on a oneway (M0280), and
// unrecognised attributes warn (M0212).
actor {
  func count(_ : Blob) : Nat = 0;
  func blob(_ : Text) : Blob = "";

  transient let wrong = { decoder = count };
  public (wrong with) func m(t : Text) : async () {};

  transient let encoding = { encoder = blob };
  public (encoding with) func ping() : () {};

  transient let extra = { decoder = count; colour = "red" };
  public (extra with) func n(_ : Nat) : async () {};
}

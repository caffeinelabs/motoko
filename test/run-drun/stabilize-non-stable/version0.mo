import Prim "mo:prim";

actor {
  transient let temporary = 1;

  func f() {
    Prim.debugPrint(debug_show (temporary));
  };

  stable let value : {
    stableField : Text;
  } = {
    stableField = "Version 0";
    nonStableField = f;
    unreachableField = -123;
  };

  stable let any : Any = f;
  stable let tuple : (Int, Any) = (0, f);
  stable let variant : { #tag : Any } = #tag f;
  stable let record : { lab : Any } = { lab = f };
  stable let vector : [Any] = [f];
  stable let array : [var Any] = [var f];
  stable let opt : ?Any = ?f;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

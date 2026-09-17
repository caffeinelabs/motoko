import Prim "mo:prim";

actor {
  transient let temporary = 1;

  func f() {
    Prim.debugPrint(debug_show (temporary));
  };

  let value : {
    stableField : Text;
  } = {
    stableField = "Version 0";
    nonStableField = f;
    unreachableField = -123;
  };

  let any : Any = f;
  let tuple : (Int, Any) = (0, f);
  let variant : { #tag : Any } = #tag f;
  let record : { lab : Any } = { lab = f };
  let vector : [Any] = [f];
  let array : [var Any] = [var f];
  let opt : ?Any = ?f;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

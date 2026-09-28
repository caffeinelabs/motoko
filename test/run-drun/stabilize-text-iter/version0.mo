import Prim "mo:prim";

actor {
  transient let temporary = 1;

  transient let textiter = "hello".chars();

  let value : {
    stableField : Text;
  } = {
    stableField = "Version 0";
    nonStableField = textiter;
    unreachableField = -123;
  };

  let any : Any = textiter;
  let tuple : (Int, Any) = (0, textiter);
  let variant : { #tag : Any } = #tag textiter;
  let record : { lab : Any } = { lab = textiter };
  let vector : [Any] = [textiter];
  let array : [var Any] = [var textiter];
  let opt : ?Any = ?textiter;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

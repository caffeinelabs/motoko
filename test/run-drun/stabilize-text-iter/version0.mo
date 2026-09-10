import Prim "mo:prim";

actor {
  transient let temporary = 1;

  transient let textiter = "hello".chars();

  stable let value : {
    stableField : Text;
  } = {
    stableField = "Version 0";
    nonStableField = textiter;
    unreachableField = -123;
  };

  stable let any : Any = textiter;
  stable let tuple : (Int, Any) = (0, textiter);
  stable let variant : { #tag : Any } = #tag textiter;
  stable let record : { lab : Any } = { lab = textiter };
  stable let vector : [Any] = [textiter];
  stable let array : [var Any] = [var textiter];
  stable let opt : ?Any = ?textiter;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

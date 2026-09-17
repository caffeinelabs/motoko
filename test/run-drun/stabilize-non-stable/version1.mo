import Prim "mo:prim";

actor {
  let value : {
    stableField : Text;
  } = {
    stableField = "Version 1";
  };

  let any : Any = null;
  let tuple : (Int, Any) = (0, null);
  let variant : { #tag : Any } = #tag null;
  let record : { lab : Any } = { lab = null };
  let vector : [Any] = [null];
  let array : [var Any] = [var null];
  let opt : ?Any = null;
  let new : () = (); // to prevent downgrade

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

import Prim "mo:prim";

actor {
  transient let temporary = 1;

  transient let blobiter = ("hello" : Blob).values();

  let value : {
    stableField : Text;
  } = {
    stableField = "Version 0";
    nonStableField = blobiter;
    unreachableField = -123;
  };

  let any : Any = blobiter;
  let tuple : (Int, Any) = (0, blobiter);
  let variant : { #tag : Any } = #tag blobiter;
  let record : { lab : Any } = { lab = blobiter };
  let vector : [Any] = [blobiter];
  let array : [var Any] = [var blobiter];
  let opt : ?Any = ?blobiter;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

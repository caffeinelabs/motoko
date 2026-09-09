import Prim "mo:prim";

actor {
  transient let temporary = 1;

  transient let blobiter = ("hello" : Blob).values();

  stable let value : {
    stableField : Text;
  } = {
    stableField = "Version 0";
    nonStableField = blobiter;
    unreachableField = -123;
  };

  stable let any : Any = blobiter;
  stable let tuple : (Int, Any) = (0, blobiter);
  stable let variant : { #tag : Any } = #tag blobiter;
  stable let record : { lab : Any } = { lab = blobiter };
  stable let vector : [Any] = [blobiter];
  stable let array : [var Any] = [var blobiter];
  stable let opt : ?Any = ?blobiter;

  public func print() : async () {
    Prim.debugPrint(debug_show (value));
  };
};

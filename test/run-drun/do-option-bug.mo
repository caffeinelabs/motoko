import P "mo:⛔";

actor a {

  public func opt() : async ?(Text,Text) {
    do ? {
      ((await async ? "a") !, (? "b") ! )
    }
  };

  public func go() : () {
    ignore await opt();
  };
}
//CALL ingress go "DIDL\x00\x00"

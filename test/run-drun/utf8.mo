import Prim "mo:⛔";
actor a {

  public func callshow(): async Text {
    let t = await show();
    Prim.debugPrint t;
    t
  };

  public func show() : async Text {
    let accum = "██    ██\n";
    Prim.debugPrint accum;
    accum
  };
};

ignore a.callshow(); //OR-CALL ingress callshow "DIDL\x00\x00"

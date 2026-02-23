//MOC-FLAG -A=M0194
import Prim "mo:⛔";
actor a {

  flexible var pending : Int = 4;

  // test that oneways can locally try/throw
  public func oneway() : () {
    Prim.debugPrint "1";
    try {
      throw (Prim.error("Error"));
      Prim.debugPrint "unreachable";
    }
    catch e { Prim.debugPrint "2"};
    pending -= 1;
  };

  // test that oneways can locally try/throw
  // using `=` syntax
  public func onewayAlt() : () =
    ignore (
      (async {
        Prim.debugPrint "3";
        try {
          throw (Prim.error("Error"));
          Prim.debugPrint "unreachable";
        }
        catch e { Prim.debugPrint "4"};
        pending -= 1;
      }) : async ()
    );


  // test that throws from oneways are silently discarded (because replies are eager)
  public func discard() : () {
    Prim.debugPrint "5";
    pending -= 1;
    throw (Prim.error("ignored"));
    Prim.debugPrint "unreachable";
  };

  // test that throws from oneways are silently discarded (because replies are eager)
  // using `=` syntax
  public func discardAlt() : () =
    ignore (
      (async {
        Prim.debugPrint "6"; 
        pending -= 1;
        throw (Prim.error("ignored"));
        Prim.debugPrint "unreachable";
      }) : async ()
    );

  // TODO test await and calls to shared functions

  public func go() : async () {
    Prim.debugPrint("A");
    oneway();
    Prim.debugPrint("B");
    onewayAlt();
    Prim.debugPrint("C");
    discard();
    Prim.debugPrint("D");
    discardAlt();
    Prim.debugPrint("E");
    while (pending > 0)
      await async ();
  };

};

ignore (a.go()); //OR-CALL ingress go 0x4449444C0000



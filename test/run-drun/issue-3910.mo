//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// test compilation of local async functions with non-trivial patterns
actor a {

  // trivial argument pattern
  func f0(r : { trap : Bool }) : async () {
    Prim.debugPrint ("f0");
  };

  // irrefutable argument pattern
  func f1({ trap : Bool }) : async () {
    Prim.debugPrint ("f1");
  };

  // refutable argument
  func f2({ trap = false : Bool }) : async () {
    Prim.debugPrint ("f2");
  };

  // desugared versions

  // trivial argument pattern
  func g0(r : { trap : Bool }) : async () = async {
    Prim.debugPrint ("g0");
  };

  // irrefutable argument pattern
  func g1({ trap : Bool }) : async () = async {
    Prim.debugPrint ("g1");
  };

  // refutable argument
  func g2({ trap = false : Bool }) : async () = async {
    Prim.debugPrint ("g2");
  };


  public func go() : async () {

    await f0({ trap = false });

    await f1({ trap = false });

    await f2({ trap = false });

    try await async {
       ignore f2({ trap = true }); // should trap, not produce async
       Prim.debugPrint ("f2() failed to trap");
       assert false;
    }
    catch e {
       Prim.debugPrint ("f2() trapped successfully");
    };

    await g0({ trap = false });

    await g1({ trap = false });

    await g2({ trap = false });

    try await async {
       ignore g2({ trap = true }); // should trap, not produce async
       Prim.debugPrint ("g2() failed to trap");
       assert false;
    }
    catch e {
       Prim.debugPrint ("g2() trapped successfully");
    };
  }

};

await a.go()//OR-CALL ingress go "DIDL\x00\x00"

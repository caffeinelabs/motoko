//MOC-FLAG -A=M0194
import Prim "mo:⛔";
// This is like local-throw.as, just not local
// (Using `await async { … }` around relevant parts

actor a {
  public func t2() : async () {
     try {
       await async {
         throw Prim.error("t2");
         assert(false);
      }
     } catch e {
          switch (Prim.errorCode(e),Prim.errorMessage(e)) {
            case (#canister_reject, "t2") { };
            case (#canister_reject, _) { assert false;};
            case (sys, _ ) { assert false;};

       }
     }
  };

  public func t3() : async () {
    try {
      await async {
        try {
          await async {
            throw Prim.error("t3");
            assert(false);
          }
        } catch e1 {
          switch (Prim.errorCode(e1), Prim.errorMessage(e1)) {
            case (#canister_reject, "t3") {
              throw Prim.error("t31");
            };
            case (#canister_reject, _) {
              assert false;
            };
            case (sys, _) {
              assert false;
            };

          }
        }
      }
    }
    catch e2 {
      switch (Prim.errorCode(e2),Prim.errorMessage(e2)) {
       case (#canister_reject, "t31") { };
       case (#canister_reject, _) {
         assert true;
       };
       case (sys, _) {
         assert false;
       };
      }
    }
  };


  public func go() : async () {
    try {
      await t2();
      Prim.debugPrint ("t2 ok");
    } catch _ {
      assert false;
    };

    try {
      await t3();
      Prim.debugPrint ("t3 ok");
    } catch _ {
      assert false;
    };
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"

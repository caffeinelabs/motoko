//MOC-FLAG -A=M0194
import P = "mo:⛔";

// TODO: test #system_fatal, #system_transient and #future somehow
// (I don't know how to trigger the first two; and the third needs a future error code)
// #system_unknown is tested in run-drun/par.mo

actor a {

  flexible let invalid = actor "bfozs-kwa73-7nadi" : actor { request() : async () };

  public func trap() : async () { assert false; };

  public func throwText(t : Text) : async () { throw P.error(t); };

  public func go() : async () {

    try {
      await throwText("ball");
    }
    catch e {
      switch (P.errorCode(e), P.errorMessage(e)) {
        case (#canister_reject, m) {
          assert (m == "ball");
          P.debugPrint("canister_reject:"#m);};
        case _ { assert false; }
      }
    };

    try {
      await trap();
    }
    catch e {
      switch (P.errorCode(e), P.errorMessage(e)) {
        case (#canister_error, m) { P.debugPrint("canister_error:" # m)};
        case _ { assert false; }
      }
    };

    try ()
      // nuthin
    catch e {
      switch (P.errorCode(e)) {
        case (#system_fatal) { assert false };
        case (#system_transient) { assert false };
        case (#system_unknown) { assert false };
        case (#destination_invalid) { assert false };
        case (#canister_error) { assert false };
        case (#canister_reject) { assert false };
        case (#future (n : Nat32)) { assert false };
        case (#call_error {err_code : Nat32}) { assert false };
      };
    };

    try {
      await invalid.request();
    }
    catch e {
      switch (P.errorCode(e), P.errorMessage(e)) {
        case (#destination_invalid, m) { P.debugPrint("destination_invalid:" # m)};
        case _ { assert false; }
      }
    }
  }
};

ignore(a.go()); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKUP run-ir
//SKIP run-low

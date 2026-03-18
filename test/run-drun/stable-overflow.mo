//MOC-FLAG -A=M0194
import P "mo:⛔";
import {grow; size; loadBlob} "stable-mem/StableMemory";

// test for correct out-of-bounds detection.
// Both ok() and bad() should fail for the same reason (but don't)
// ok() reports RTS error: region access out of bounds, caught by RTS
// bad() reports: "stable memory out of bounds", not caught by RTS but by IC

// I think one could exploit this bound check failure to break isolation between regions...

actor {
    let n = grow(1);
    assert n == 0;
    assert size() == 1;

    public func ok() : async () {
        ignore loadBlob(0xFFFF, 0);
        ignore loadBlob(0xFFFF, 1);
        ignore loadBlob(0xFFFE, 2);
        ignore loadBlob(0xFFFE, 1);
        ignore loadBlob(0x0,0x1_0000);
    };

    public func trap1() : async () {
      ignore loadBlob(0xFFFF_FFFF_FFFF_FFFF, 0);
      assert false
    };

    public func trap2() : async () {
      ignore loadBlob(0xFFFF, 2);
      assert false
    };

    public func trap3() : async () {
      ignore loadBlob(0xFFFE, 3);
      assert false
    };

    public func trap4() : async () {
      ignore loadBlob(0x0, 0x1_0001);
      assert false;
    };

    public func trap5() : async () {
      ignore loadBlob(0x1_0000, 0);
      assert false;
    };

    public func trap6() : async () {
      ignore loadBlob(0x1_0000, 1);
      assert false;
    };

    public func go() : async () {

      try await ok()
      catch e {
        assert false;
      };

      try await trap1()
      catch e {
         P.debugPrint(P.errorMessage e);
      };

      try await trap2()
      catch e {
        P.debugPrint(P.errorMessage e);
      };

      try await trap3()
      catch e {
        P.debugPrint(P.errorMessage e);
      };

      try await trap4()
      catch e {
        P.debugPrint(P.errorMessage e);
      };

      try await trap5()
      catch e {
        P.debugPrint(P.errorMessage e);
      };

      try await trap6()
      catch e {
        P.debugPrint(P.errorMessage e);
      };

    }

}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"


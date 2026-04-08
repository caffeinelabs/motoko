//MOC-FLAG -A=M0194
import Prim "mo:⛔";
import Lib "gc-trigger-acl/C";
// test __motoko_gc_trigger can only be invoked from self or controller
// using an actor class instance
actor Self {

  type GC = actor {
    __motoko_gc_trigger: () -> async ()
  };

  func toGC(a : actor {}) : GC {
    let gc = actor (debug_show (Prim.principalOfActor(a))) : GC;
    gc;
  };

  public func go() : async () {

    let GC = toGC(Self);
    // test call from self
    do {
      try {
        await GC.__motoko_gc_trigger();
        Prim.debugPrint("Self.gc_trigger()");
      }
      catch e {
        assert false;
      }
    };

    let c = await (Lib.C(GC));

    // test call from controller (Self is a controller of c)
    do {
      try {
        await toGC(c).__motoko_gc_trigger();
        Prim.debugPrint("controlee.gc_trigger()");
      }
      catch e {
        assert false;
      }
    };

    // test callback from non-controller (c is not a controller of Self nor GC)
    do {
      try {
        await c.callback();
        assert false;
      }
      catch e {
        Prim.debugPrint(Prim.errorMessage(e));
      }
    }

  }
}

//CALL ingress go "DIDL\x00\x00"

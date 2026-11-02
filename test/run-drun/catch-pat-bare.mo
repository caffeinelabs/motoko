// `catch` takes the same unparenthesized patterns as `case`
import Prim "mo:prim";

actor {
  func fail() : async () { throw Prim.error("boom") };

  public func go() : async () {
    try { await fail() } catch e : Error {
      Prim.debugPrint(Prim.errorMessage(e))
    };
    try { await fail() } catch _ {
      Prim.debugPrint("caught")
    }
  }
};

//CALL ingress go "DIDL\x00\x00"

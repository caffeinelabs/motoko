
import Prim "mo:prim";

actor {
  public shared func test() : async () {
    let _signer = Prim.callerInfoSigner<system>();
    let _info = Prim.callerInfoData<system>();
  };
}

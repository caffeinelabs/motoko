
import Prim "mo:prim";

actor {
  let _signer = Prim.callerInfoSigner<system>();
  let _info = Prim.callerInfoData<system>();
}

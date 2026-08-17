//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";

// The blob-like payloads must bound their LEB128 length against the bytes left
// in the message before allocating from it (`principal` always did, capping at
// 29 bytes). Both payloads below claim ~3.25 GiB and then stop, so decoding has
// to fail on the length, not grow the heap to the claimed size first.

actor {

  func deserBlob(x : Blob) : Blob = (prim "deserialize" : Blob -> Blob) x;
  func deserText(x : Blob) : Text = (prim "deserialize" : Blob -> Text) x;

  public func go () : async () {
    try {
      await async { ignore deserBlob "DIDL\01\6d\7b\01\00\80\80\80\80\0d" };
      assert false
    }
    catch e {
      Prim.debugPrint(Prim.errorMessage(e));
    };
    try {
      await async { ignore deserText "DIDL\00\01\71\80\80\80\80\0d" };
      assert false
    }
    catch e {
      Prim.debugPrint(Prim.errorMessage(e));
    }
  }
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"

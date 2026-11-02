//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";

// The blob-like payloads must bound their LEB128 length against the bytes left
// in the message before allocating from it (`principal` always did, capping at
// 29 bytes). The window shrinks as arguments are read, so the lengths claimed
// across a message cannot add up to more than the message itself.

actor {

  func deserBlob(x : Blob) : Blob = (prim "deserialize" : Blob -> Blob) x;
  func deserText(x : Blob) : Text = (prim "deserialize" : Blob -> Text) x;
  func deser3(x : Blob) : (Blob, Blob, Blob) =
    (prim "deserialize" : Blob -> (Blob, Blob, Blob)) x;
  func deser7(x : Blob) : (Blob, Blob, Blob, Blob, Blob, Blob, Blob) =
    (prim "deserialize" : Blob -> (Blob, Blob, Blob, Blob, Blob, Blob, Blob)) x;

  // seven blobs, each claiming 0x1F0000 = 1.9 MiB (so each on its own stays
  // below the message size limit, while together they are far above it), with
  // no payload bytes at all
  let seven : [Nat8] = [0x44,0x49,0x44,0x4c, 0x01,0x6d,0x7b,
                        0x07,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                        0x80,0x80,0xfc,0x00, 0x80,0x80,0xfc,0x00, 0x80,0x80,0xfc,0x00,
                        0x80,0x80,0xfc,0x00, 0x80,0x80,0xfc,0x00, 0x80,0x80,0xfc,0x00,
                        0x80,0x80,0xfc,0x00];

  // three blobs, of which the first honestly carries its 900 bytes; the second
  // then claims 900 more, which the shrunken window no longer holds
  let prefix : [Nat8] = [0x44,0x49,0x44,0x4c, 0x01,0x6d,0x7b, 0x03,0x00,0x00,0x00, 0x84,0x07];
  let cumulative : [Nat8] = Prim.Array_tabulate<Nat8>(prefix.size() + 900 + 2, func i =
    if (i < prefix.size()) prefix[i]
    else if (i < prefix.size() + 900) 0x41
    else if (i == prefix.size() + 900) 0x84
    else 0x07);

  // the same shape, honestly encoded, must still decode
  let honest : [Nat8] = [0x44,0x49,0x44,0x4c, 0x01,0x6d,0x7b, 0x03,0x00,0x00,0x00,
                         0x03,0x41,0x41,0x41, 0x03,0x42,0x42,0x42, 0x03,0x43,0x43,0x43];

  func expectTrap(what : Text, decode : () -> ()) : async () {
    try {
      await async { decode() };
      assert false
    }
    catch e {
      Prim.debugPrint(what # ": " # Prim.errorMessage(e));
    }
  };

  public func go () : async () {
    // claims ~3.25 GiB and stops
    await expectTrap("blob", func () = ignore deserBlob "DIDL\01\6d\7b\01\00\80\80\80\80\0d");
    await expectTrap("text", func () = ignore deserText "DIDL\00\01\71\80\80\80\80\0d");
    await expectTrap("seven", func () = ignore deser7 (Prim.arrayToBlob seven));
    await expectTrap("cumulative", func () = ignore deser3 (Prim.arrayToBlob cumulative));
    Prim.debugPrint(debug_show (deser3 (Prim.arrayToBlob honest)));
  }
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"

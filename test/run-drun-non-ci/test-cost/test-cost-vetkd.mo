import Prim "mo:⛔";

actor client {
  func print(t : Text) = Prim.debugPrint("client: " # t);

  type Key = { name : Text; curve : VetKDCurve };
  type VetKDCurve = { #bls12_381 };
  type Args = {
    input : Blob;
    context : Blob;
    transport_public_key : Blob;
    key_id : Key;
  };
  let ic00 = actor "aaaaa-aa" : actor {
    vetkd_derive_key : shared Args -> async { encrypted_key : Blob };
  };
  func encodeCurve(curve : VetKDCurve) : Nat32 = switch curve {
    case (#bls12_381) 0;
  };

  public shared ({ caller }) func go() : async () {
    let fakeContext = Prim.arrayToBlob([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let fakeTransportPublicKey = Prim.arrayToBlob([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    let key : Key = { name = "test_key_1"; curve = #bls12_381 };
    let args : Args = {
      input = Prim.blobOfPrincipal(caller);
      context = fakeContext;
      transport_public_key = fakeTransportPublicKey;
      key_id = key;
    };

    await test(args);
  };

  func test(args : Args) : async () {
    let (code, cost) = Prim.costVetkdDeriveKey(args.key_id.name, encodeCurve(args.key_id.curve));
    print(debug_show (code, cost) # " -- vetkd derive key cost");
    assert code == 0 and cost > 0;

    printCycles();
    let { encrypted_key } = await (with cycles = cost) ic00.vetkd_derive_key(args);
    print("encrypted_key: " # debug_show (encrypted_key));
    printCycles();

    // Try the same args with less cycles, it should fail
    try {
      let _ = await (with cycles = cost - 1) ic00.vetkd_derive_key(args);
      assert false; // Should not happen
    } catch (e) {
      assert Prim.errorCode(e) == #canister_reject;
      print("error message: " # debug_show (Prim.errorMessage(e)));
    };
    print("---");
  };

  func printCycles() {
    print("Cycles.balance()   = " # debug_show (Prim.cyclesBalance()));
    print("Cycles.available() = " # debug_show (Prim.cyclesAvailable()));
    print("Cycles.refunded()  = " # debug_show (Prim.cyclesRefunded()));
  };
};

client.go(); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low

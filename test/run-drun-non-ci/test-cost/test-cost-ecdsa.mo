import Prim "mo:⛔";

actor client {
  func print(t : Text) = Prim.debugPrint("client: " # t);

  type Key = { name : Text; curve : EcdsaCurve };
  type EcdsaCurve = { #secp256k1 };
  type Args = {
    key_id : Key;
    derivation_path : [Blob];
    message_hash : Blob;
  };
  let ic00 = actor "aaaaa-aa" : actor {
    sign_with_ecdsa : shared Args -> async { signature : Blob };
  };
  func encodeCurve(curve : EcdsaCurve) : Nat32 = switch curve {
    case #secp256k1 { 0 }
  };

  public shared ({ caller }) func go() : async () {
    let fakeMessageHash = Prim.arrayToBlob([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    let key : Key = { name = "test_key_1"; curve = #secp256k1 };
    let args : Args = {
      message_hash = fakeMessageHash;
      derivation_path = [Prim.blobOfPrincipal(caller)];
      key_id = key;
    };

    // Local Output:
    // client: test args:
    // {derivation_path = ["\BF\EB\AF\3B\30\B2\AA\72\63\D3\B7\98\4D\A1\C6\81\16\61\43\D5\70\D0\3A\62\E0\1C\2B\47\02"]; key_id = {curve = #secp256k1; name = "test_key_1"}; message_hash = "\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D\1E\1F\20"}
    // client: (0, 26_153_846_153) -- sign with ecdsa cost
    // client: Cycles.balance()   = 2_916_759_689_901
    // client: Cycles.available() = 0
    // client: Cycles.refunded()  = 0
    // client: signature: "\01\4A\60\B1\C8\0F\A6\8B\84\9E\FD\7C\49\4B\65\00\46\6D\96\80\5C\BE\25\81\FB\F3\B3\0F\54\AE\A6\BC\17\3B\E0\95\B7\3A\E9\70\3E\85\3F\6C\87\64\E3\1F\5C\A6\07\DB\53\73\5F\29\74\65\CD\7F\0F\D6\FF\06"
    // client: Cycles.balance()   = 2_890_600_025_727
    // client: Cycles.available() = 0
    // client: Cycles.refunded()  = 0
    // client: 26_159_664_174 -- Cycles.balance() diff
    // client: error message: "sign_with_ecdsa request sent with 26_153_846_152 cycles, but 26_153_846_153 cycles are required."
    await test(args);
  };

  func test(args : Args) : async () {
    print("test args:\n" # debug_show args);
    let (code, cost) = Prim.costSignWithEcdsa(args.key_id.name, encodeCurve(args.key_id.curve));
    print(debug_show (code, cost) # " -- sign with ecdsa cost");
    assert code == 0 and cost > 0;

    printCycles();
    let { signature } = await (with cycles = cost) ic00.sign_with_ecdsa(args);
    print("signature: " # debug_show (signature));
    printCycles();

    // Try the same args with less cycles, it should fail
    try {
      let _ = await (with cycles = cost - 1) ic00.sign_with_ecdsa(args);
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

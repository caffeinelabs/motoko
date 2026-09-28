import Prim "mo:⛔";

actor client {
  func print(t : Text) = Prim.debugPrint("client: " # t);

  type SchnorrAlgorithm = {
    #ed25519;
    #bip340secp256k1;
  };
  type Key = {
    algorithm : SchnorrAlgorithm;
    name : Text;
  };
  type Args = {
    key_id : Key;
    derivation_path : [Blob];
    message : Blob;
  };
  let ic00 = actor "aaaaa-aa" : actor {
    sign_with_schnorr : shared Args -> async { signature : Blob };
  };
  func encodeAlgorithm(algorithm : SchnorrAlgorithm) : Nat32 = switch algorithm {
    case #bip340secp256k1 { 0 }
    case #ed25519 { 1 }
  };

  public shared ({ caller }) func go() : async () {
    let fakeMessage : Blob = Prim.arrayToBlob([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    let key : Key = { name = "test_key_1"; algorithm = #bip340secp256k1 };
    let args : Args = {
      message = fakeMessage;
      derivation_path = [Prim.blobOfPrincipal(caller)];
      key_id = key;
    };

    // Local Output:
    // client: test args:
    // {derivation_path = ["\BF\EB\AF\3B\30\B2\AA\72\63\D3\B7\98\4D\A1\C6\81\16\61\43\D5\70\D0\3A\62\E0\1C\2B\47\02"]; key_id = {algorithm = #bip340secp256k1; name = "test_key_1"}; message = "\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D\1E\1F\20"}
    // client: (0, 26_153_846_153) -- sign with schnorr cost
    // client: Cycles.balance()   = 2_916_757_439_901
    // client: Cycles.available() = 0
    // client: Cycles.refunded()  = 0
    // client: signature: "\01\72\50\A8\5B\2A\5B\79\F8\34\37\9F\07\78\3C\C2\91\45\4C\04\6C\BC\54\13\31\EC\98\73\96\75\36\A6\98\B0\8F\79\11\03\1A\41\FE\D2\D5\4B\5A\F8\46\CD\F2\D4\40\96\47\5E\A6\6D\B3\7C\00\BF\46\4D\02\5E"
    // client: Cycles.balance()   = 2_890_597_766_872
    // client: Cycles.available() = 0
    // client: Cycles.refunded()  = 0
    // client: 26_159_673_029 -- Cycles.balance() diff
    // client: error message: "sign_with_schnorr request sent with 26_153_846_152 cycles, but 26_153_846_153 cycles are required."
    // client: ---
    await test(args);
  };

  func test(args : Args) : async () {
    print("test args:\n" # debug_show args);
    let (code, cost) = Prim.costSignWithSchnorr(args.key_id.name, encodeAlgorithm(args.key_id.algorithm));
    print(debug_show (code, cost) # " -- sign with schnorr cost");
    assert code == 0 and cost > 0;

    printCycles();
    let { signature } = await (with cycles = cost) ic00.sign_with_schnorr(args);
    print("signature: " # debug_show (signature));
    printCycles();

    // Try the same args with less cycles, it should fail
    try {
      let _ = await (with cycles = cost - 1) ic00.sign_with_schnorr(args);
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

//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  type Record = { value : Text; count : Nat64 };
  type Result<T> = { #ok : T; #err : Text };
  transient let methods = { decode = Prim.fixtureDecode };

  func decode(text : Text) : async* Result<Record> {
    let before = methods.decode<Record>(text);
    await async {};
    assert before == methods.decode<Record>(text);
    before
  };

  public func check() : async () {
    let full = await* decode("through async");
    let projected : Result<{ value : Text }> = full;
    assert projected == #ok { value = "through async" };
    assert full == #ok { value = "through async"; count = 13 };
    assert Prim.fixtureDecode<Text>("after await") == #ok "after await";
    Prim.debugPrint("typed plugin specialization survived async lowering");
  };
};

//CALL ingress check "DIDL\x00\x00"

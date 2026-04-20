//MOC-FLAG --package core $MOTOKO_CORE
import Prim "mo:prim";
import Debug "mo:core/Debug";
import Principal "mo:core/Principal";
import Runtime "mo:core/Runtime";
import Option "mo:core/Option";

actor {

  type EnvVar = { name : Text; value : Text };
  transient let ic = actor "aaaaa-aa" : actor {
    update_settings : shared {
      canister_id : Principal;
      settings : {
        environment_variables : ?[EnvVar];
      };
    } -> async ();
  };

  public shared func setTrustedCaller() : async () {
    let self = Prim.getSelfPrincipal<system>();
    await ic.update_settings({
      canister_id = self;
      settings = {
        environment_variables = ?[{
          name = "trusted_attribute_signers";
          value = "rdmx6-jaaaa-aaaaa-aaadq-cai";
        }];
      };
    });
  };

  public shared func test() : async () {
    let signer = Prim.callerInfoSigner<system>();
    let trustedSigners = Runtime.envVar<system>("trusted_attribute_signers");
    switch (signer.size() != 0, trustedSigners) {
      case (true, ?trustedSigners) {
        if (Principal.fromBlob(signer) != Principal.fromText(trustedSigners)) {
          Runtime.trap("untrusted signer");
        };
      };
      case _ {
        Runtime.trap("Signer or trusted signers not available");
      };
    };
    let info = Prim.callerInfoData<system>();
    assert info == ("\00\00\00" : Blob);
  };

  // II canister principal extracted from the canister sig public key (ICRC-3 test vectors)
  let iiSignerBlob : Blob = "\ff\ff\ff\ff\ff\e0\00\00\01\01";

  type Icrc3Value = {
    #Nat : Nat;
    #Int : Int;
    #Blob : Blob;
    #Text : Text;
    #Array : [Icrc3Value];
    #Map : [(Text, Icrc3Value)];
  };

  func lookupText(map : [(Text, Icrc3Value)], key : Text) : ?Text {
    for ((k, v) in map.vals()) {
      if (k == key) {
        switch v {
          case (#Text t) { return ?t };
          case _ {};
        };
      };
    };
    null;
  };

  public shared func checkCallerInfo() : async () {
    let signer = Prim.callerInfoSigner<system>();
    if (signer.size() == 0) {
      Runtime.trap("no signer");
    };
    assert signer == iiSignerBlob;

    let data = Prim.callerInfoData<system>();
    let ?map : ?Icrc3Value = from_candid (data) else Runtime.trap("invalid candid");
    let #Map(entries) = map else Runtime.trap("expected Map");

    let ?origin = lookupText(entries, "implicit:origin") else Runtime.trap("missing origin");
    assert origin == "https://some-dapp.com";

    switch (lookupText(entries, "email")) {
      case (?email) { Debug.print(email) };
      case _ {};
    };
    switch (lookupText(entries, "openid:https://accounts.google.com:email")) {
      case (?email) { Debug.print(email) };
      case _ {};
    };

  };
};

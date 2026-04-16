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
    await ic.update_settings({ canister_id = self; settings = {
      environment_variables = ?[{
        name = "trusted_attribute_signers";
        value = "rdmx6-jaaaa-aaaaa-aaadq-cai"
      }] }
    });
  };

  public shared func test() : async () {
    let signer = Prim.callerInfoSigner<system>();
    let trustedSigners = Runtime.envVar<system>("trusted_attribute_signers");
    switch (signer, trustedSigners) {
      case (?signer, ?trustedSigners) {
        if (signer != Principal.fromText(trustedSigners)) {
          Runtime.trap("untrusted signer")
        };
      };
      case _ {
        Runtime.trap("Signer or trusted signers not available")
      };
    };
    let info = Prim.callerInfoData<system>();
    assert info == ("\00\00\00" : Blob);
  };
}

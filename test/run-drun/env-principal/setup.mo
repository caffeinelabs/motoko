import { principalOfActor } = "mo:⛔";

persistent actor Self {
  type EnvVar = { name : Text; value : Text };

  transient let ic = actor "aaaaa-aa" : actor {
    update_settings : shared {
      canister_id : Principal;
      settings : { environment_variables : ?[EnvVar] };
    } -> async ();
  };

  public func run() : async () {
    await ic.update_settings({
      canister_id = principalOfActor Self;
      settings = { environment_variables = ?[{ name = "management"; value = "aaaaa-aa" }] };
    });
  };
};

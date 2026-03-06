//MOC-FLAG --actor-idl env-principal
//MOC-FLAG --actor-env-alias management management

import { principalOfActor; debugPrint } = "mo:⛔";
import Management "canister:management";

persistent actor Self {
  public func go() : async () {
    let status = await Management.canister_status { canister_id = principalOfActor Self };
    let vars = status.settings.environment_variables;
    debugPrint (debug_show vars);
    assert vars == [{ name = "management"; value = "aaaaa-aa" }];
  };
};

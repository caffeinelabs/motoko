import Prim "mo:⛔";

actor Self {
  type EnvVar = { name : Text; value : Text };

  let ic = actor "aaaaa-aa" : actor {
    canister_status : shared {
      canister_id : Principal;
    } -> async {
      settings : {
        environment_variables : [EnvVar];
      };
    };
    update_settings : shared {
      canister_id : Principal;
      settings : {
        environment_variables : ?[EnvVar];
      };
    } -> async ();
  };

  func getEnvVars() : async* [EnvVar] {
    (await ic.canister_status({ canister_id = Prim.principalOfActor(Self) })).settings.environment_variables;
  };

  func setEnvVars(vars : [EnvVar]) : async* () {
    await ic.update_settings({
      canister_id = Prim.principalOfActor(Self);
      settings = { environment_variables = ?vars };
    });
    let currentVars = await* getEnvVars();
    if (currentVars != vars) {
      Prim.debugPrint("Unexpected environment variables: " # debug_show currentVars);
      assert false;
    };
  };

  public func run() : async () {
    await* setEnvVars([
      {
        name = "TEST_ENV_VAR_NAME";
        value = "Test environment variable value";
      },
      {
        name = "key";
        value = "value";
      },
    ]);
    Prim.debugPrint(debug_show Prim.envVarNames<system>());
    assert Prim.envVarNames<system>() == ["TEST_ENV_VAR_NAME", "key"];
    assert Prim.envVar<system>("TEST_ENV_VAR_NAME") == ?"Test environment variable value";
    assert Prim.envVar<system>("key") == ?"value";
    assert Prim.envVar<system>("OTHER_ENV_VAR_NAME") == null;
    assert Prim.envVar<system>("") == null;

    await* setEnvVars([{
      name = "NEW_ENV_VAR";
      value = "NEW_VALUE";
    }]);
    Prim.debugPrint(debug_show Prim.envVarNames<system>());
    assert Prim.envVarNames<system>() == ["NEW_ENV_VAR"];
    assert Prim.envVar<system>("NEW_ENV_VAR") == ?"NEW_VALUE";
    assert Prim.envVar<system>("TEST_ENV_VAR_NAME") == null;
    assert Prim.envVar<system>("key") == null;

    await* setEnvVars([]);
    Prim.debugPrint(debug_show Prim.envVarNames<system>());
    assert Prim.envVarNames<system>() == [];
    assert Prim.envVar<system>("TEST_ENV_VAR_NAME") == null;
    assert Prim.envVar<system>("NEW_ENV_VAR") == null;
  };

};

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP wasm-run

//CALL ingress run "DIDL\x00\x00"

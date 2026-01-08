import Prim "mo:⛔";
import { ic } "mo:ic";

import (v0 : Blob) = "blob:file:wasms/v0.wasm";
import (v1 : Blob) = "blob:file:wasms/v1.wasm";
import (v2 : Blob) = "blob:file:wasms/v2.wasm";
import (v3 : Blob) = "blob:file:wasms/v3.wasm";
import (v4 : Blob) = "blob:file:wasms/v4.wasm";
import (v5 : Blob) = "blob:file:wasms/v5.wasm";
import (v6 : Blob) = "blob:file:wasms/v6.wasm";
import (v7 : Blob) = "blob:file:wasms/v7.wasm";
import (v8 : Blob) = "blob:file:wasms/v8.wasm";
import (v9 : Blob) = "blob:file:wasms/v9.wasm";
import (v10 : Blob) = "blob:file:wasms/v10.wasm";
import (v11 : Blob) = "blob:file:wasms/v11.wasm";
import (v12 : Blob) = "blob:file:wasms/v12.wasm";
import (v13 : Blob) = "blob:file:wasms/v13.wasm";
import (v14 : Blob) = "blob:file:wasms/v14.wasm";
import (v15 : Blob) = "blob:file:wasms/v15.wasm";
import (v16 : Blob) = "blob:file:wasms/v16.wasm";
import (v17 : Blob) = "blob:file:wasms/v17.wasm";
import (v18 : Blob) = "blob:file:wasms/v18.wasm";
import (v19 : Blob) = "blob:file:wasms/v19.wasm";
import (v20 : Blob) = "blob:file:wasms/v20.wasm";

persistent actor {

  let canisters = [v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15, v16, v17, v18, v19, v20];

  public func install_all() : async () {
    let controller = Prim.getSelfPrincipal<system>();
    Prim.debugPrint(debug_show {controller});
    let result = await ic.create_canister {
      settings = ?{
        controllers = ?[controller];
        compute_allocation = null;
        memory_allocation = null;
        freezing_threshold = null;
        reserved_cycles_limit = null;
        log_visibility = null;
        wasm_memory_limit = null;
        wasm_memory_threshold = null;
      };
      sender_canister_version = null;
    };

    let canister_id = result.canister_id;
    Prim.debugPrint(debug_show {canister_id});
    let res = await ic.install_code {
      mode = #install;
      canister_id = canister_id;
      wasm_module = v0;
      arg = "";
      sender_canister_version = null;
    };

    Prim.debugPrint(debug_show res);

    let installModalities = {
      mode = #upgrade(
        ?{
          wasm_memory_persistence = ?#keep;
          skip_pre_upgrade = null; // or ?true or ?false
        }
      );
      canister_id = canister_id;
      arg = "" : Blob;
      sender_canister_version = null;
    };

    for (wasm_module in canisters.values()) {
      Prim.debugPrint "upgrading";
      await ic.install_code { installModalities with wasm_module };
    }
  };
};

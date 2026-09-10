// Provides provenance for the committed classical `old.wasm` fixture used by
// `migration-paths.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:⛔";
import Cycles = "../cycles/cycles";
import TestCanister "test_canister";

actor installer {
    stable var testCanister: ?TestCanister.TestCanister = null;

    public func install() : async () {
        if (Cycles.balance() == 0) {
            await Cycles.provisional_top_up_actor(installer, 100_000_000_000_000);
        };

        Cycles.add<system>(2_000_000_000_000);
        testCanister := ?(await TestCanister.TestCanister());

        Prim.debugPrint("Installed");
    };

    public func upgrade() : async () {
         switch testCanister {
            case null Prim.trap("null canister");
            case (?canister) {
                ignore await (system TestCanister.TestCanister)(#upgrade canister)();
                Prim.debugPrint("Upgraded (default)");
            }
         }
    };

    public func upgrade_keep_main_memory() : async () {
         switch testCanister {
            case null Prim.trap("null canister");
            case (?canister) {
                ignore await (system TestCanister.TestCanister)(#upgrade_with_persistence { wasm_memory_persistence =  #keep; canister })();
                Prim.debugPrint("Upgraded (keep main memory)");
            }
         }
    };

    public func upgrade_replace_main_memory() : async () {
         switch testCanister {
            case null Prim.trap("null canister");
            case (?canister) {
                ignore await (system TestCanister.TestCanister)(#upgrade_with_persistence { wasm_memory_persistence = #replace; canister })();
                Prim.debugPrint("Upgraded (replace main memory)");
            }
         }
    };
};

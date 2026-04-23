//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:⛔";
import TestCanister "test_canister";

actor {
    stable var testCanister : ?TestCanister.TestCanister = null;

    public func upgrade() : async () {
        let canister = testCanister ?? Prim.trap("null canister");
        ignore await (system TestCanister.TestCanister)(#upgrade canister)();
        Prim.debugPrint("Upgraded (default)");
    };

    public func upgrade_keep_main_memory() : async () {
        let canister = testCanister ?? Prim.trap("null canister");
        ignore await (system TestCanister.TestCanister)(#upgrade_with_persistence { wasm_memory_persistence = #keep; canister })();
        Prim.debugPrint("Upgraded (keep main memory)");
    };

    public func upgrade_replace_main_memory() : async () {
        let canister = testCanister ?? Prim.trap("null canister");
        ignore await (system TestCanister.TestCanister)(#upgrade_with_persistence { wasm_memory_persistence = #replace; canister })();
        Prim.debugPrint("Upgraded (replace main memory)");
    };

    type GraphCopy = actor {
        __motoko_stabilize_before_upgrade : () -> async ();
    };

    func getGraphCopy(a : actor {}) : GraphCopy {
        let graphCopy = actor (debug_show (Prim.principalOfActor(a))) : GraphCopy;
        graphCopy;
    };

    public func initiate_graph_copy() : async () {
        let canister = testCanister ?? Prim.trap("null canister");
        let graphCopy = getGraphCopy(canister);
        await graphCopy.__motoko_stabilize_before_upgrade();
        Prim.debugPrint("Graph copy");
    };
};

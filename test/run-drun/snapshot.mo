//MOC-FLAG -A=M0270
import Prim "mo:prim";
import Cycles "cycles/cycles";
import TestActor "snapshot/test-actor";

actor MainActor {
    type snapshot = {
        id : Blob;
        taken_at_timestamp : Nat64;
        total_size : Nat64;
    };

    type ManagementCanister = actor {
        take_canister_snapshot : {
            canister_id : Principal;
            replace_snapshot : ?Blob;
        } -> async snapshot;

        load_canister_snapshot : {
            canister_id : Principal;
            snapshot_id : Blob;
            sender_canister_version : ?Nat64;
        } -> async ();

        list_canister_snapshots : {
            canister_id : Principal;
        } -> async [snapshot];

        delete_canister_snapshot : {
            canister_id : Principal;
            snapshot_id : Blob;
        } -> async ();
    };

    func getManagementCanister() : ManagementCanister {
        actor "aaaaa-aa" : ManagementCanister;
    };

    public func go() : async () {
        if (Cycles.balance() == 0) {
            await Cycles.provisional_top_up_actor(MainActor, 100_000_000_000_000);
        };
        Cycles.add<system>(2_000_000_000_000);
        let testActor = await TestActor.TestActor();
        let testCanisterId = Prim.principalOfActor(testActor);
        let manager = getManagementCanister();
        let snapshot1 = await manager.take_canister_snapshot({
            canister_id = testCanisterId;
            replace_snapshot = null : ?Blob;
        });
        Prim.debugPrint("First snapshot taken");
        Prim.debugPrint("Test actor version: " # debug_show (await testActor.getVersion()));
        await testActor.breakNextUpgrade();
        try {
            ignore await (system TestActor.TestActor)(#upgrade testActor)();
            Prim.trap("Upgrade was not trapping");
        } catch (_) {
            Prim.debugPrint("Upgrade failed as expected");
        };
        await manager.load_canister_snapshot({
            canister_id = testCanisterId;
            snapshot_id = snapshot1.id;
            sender_canister_version = null : ?Nat64;
        });
        Prim.debugPrint("First snapshot restored");
        Prim.debugPrint("Test actor version: " # debug_show (await testActor.getVersion()));
        ignore await (system TestActor.TestActor)(#upgrade testActor)();
        Prim.debugPrint("Upgrade succeeded");
        Prim.debugPrint("Test actor version: " # debug_show (await testActor.getVersion()));
        let list = await manager.list_canister_snapshots({
            canister_id = testCanisterId;
        });
        assert (list[0] == snapshot1);
        let snapshot2 = await manager.take_canister_snapshot({
            canister_id = testCanisterId;
            replace_snapshot = ?snapshot1.id;
        });
        ignore await (system TestActor.TestActor)(#upgrade testActor)();
        Prim.debugPrint("Test actor version: " # debug_show (await testActor.getVersion()));
        await manager.load_canister_snapshot({
            canister_id = testCanisterId;
            snapshot_id = snapshot2.id;
            sender_canister_version = null : ?Nat64;
        });
        Prim.debugPrint("Test actor version: " # debug_show (await testActor.getVersion()));
        await manager.delete_canister_snapshot({
            canister_id = testCanisterId;
            snapshot_id = snapshot2.id;
        });
        Prim.debugPrint("Snapshot deleted");
    };
};

//CALL ingress go "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low

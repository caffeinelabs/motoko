// Only works with incremental GC. 
// Use:
// ```
// export EXTRA_MOC_ARGS="--incremental-gc"
// run-test -d actor64.drun
// ```
import Prim "mo:prim";

actor {
    let MB = 1024 * 1024;
    let GB = 1024 * MB;

    Prim.debugPrint("Hello Wasm64 Motoko on IC!");

    func trace() {
        Prim.debugPrint("Heap size: " # debug_show (Prim.rts_heap_size() / GB) # " GB");
    };

    type Node = ?{ value : [var Nat]; next : Node };
    var first : Node = null;

    public func allocate() : async () {
        let value = Prim.Array_init<Nat>(128 * MB, 0);
        first := ?{ value; next = first };
        trace();
    };
};

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"
//CALL ingress allocate "DIDL\x00\x00"

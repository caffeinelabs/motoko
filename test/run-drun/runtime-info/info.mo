import Prim "mo:prim";

module {
    public type TextList = ?(Text, TextList);
    public type RuntimeInformation = {
        compilerVersion : Text;
        rtsVersion : Text;
        garbageCollector : Text;
        sanityChecks : Bool;
        memorySize : Nat;
        heapSize : Nat;
        totalAllocation : Nat;
        reclaimed : Nat;
        maxLiveSize : Nat;
        stableMemorySize : Nat;
        logicalStableMemorySize : Nat;
        maxStackSize : Nat;
        callbackTableCount : Nat;
        callbackTableSize : Nat;
        version : TextList;
    };

    public type ActorIntrospection = actor {
        __motoko_runtime_information : () -> async RuntimeInformation;
    };

    public func introspect(a : actor {}) : ActorIntrospection {
        (actor (debug_show (Prim.principalOfActor(a))) : ActorIntrospection);
    };
};

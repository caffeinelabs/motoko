import Callee "./callee";

actor class Caller(_callee: Principal) {
    public type CalleeActor = actor {
        add1: () -> async Nat;
        get: query () -> async Nat;
    };

    private let calleeHandler : CalleeActor = actor(debug_show(_callee));

    public func add1() : async Nat {
        await calleeHandler.add1()
    };

    public func get() : async Nat {
        await calleeHandler.get()
    };
};

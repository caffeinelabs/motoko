import Principal "mo:core/Principal";
import Set "mo:core/pure/Set";
import Error "mo:core/Error";

persistent actor {

    // Create set to record principals
    var principals : Set.Set<Principal> = Set.empty();

    // Check if principal is recorded
    public shared query(msg) func isRecorded() : async Bool {
        let caller = msg.caller;
        Set.contains(principals, Principal.compare, caller);
    };

    // Record a new principal
    public shared(msg) func recordPrincipal() : async () {
        let caller = msg.caller;
        if (Principal.isAnonymous(caller)) {
            throw Error.reject("Anonymous principal not allowed");
        };

        principals := Set.add(principals, Principal.compare, caller)
    };
};

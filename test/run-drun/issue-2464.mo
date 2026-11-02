import Prim "mo:⛔";
import Caller "issue-2464/caller";

actor class Maker() {
    /// Append the values of two input arrays
    func append<A>(xs : [A], ys : [A]) : [A] {
      switch(xs.size(), ys.size()) {
        case (0, 0) { []; };
        case (0, _) { ys; };
        case (_, 0) { xs; };
        case (xsSize, ysSize) {
         Prim.Array_tabulate<A>(xsSize + ysSize, func (i : Nat) : A {
           if (i < xsSize) {
             xs[i];
           } else {
             ys[i - xsSize];
           };
         });
        };
      };
    };

    private var callers : [Principal] = [];

    public func newCaller(callee: Principal) : async Principal {
        let callerActor : actor {} = await Caller.Caller(callee);
        let callerId = Prim.principalOfActor(callerActor);
        callers := append<Principal>(callers, [callerId]);

        callerId
    };

    public query func getcallers() : async [Principal] {
        callers
    };
};

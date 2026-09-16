//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
import Prim "mo:prim";

actor {
    type Node<T> = {
        value : T;
        left : ?Node<T>;
        right : ?Node<T>;
    };

    stable var root : ?Node<Nat> = null;

    func double(value : Nat) {
        root := ?{
            value;
            left = root;
            right = root;
        };
    };


    public func run() : async () {
        var size = 0;
        while (size < 25) {
            double(size);
            size += 1;
        };
    };

    system func postupgrade() {
        Prim.debugPrint("Upgraded!");
    };
};

//CALL ingress run "DIDL\x00\x00"
//CALL ingress __motoko_stabilize_before_upgrade "DIDL\x00\x00"
//CALL upgrade ""

//SKIP run-low
//SKIP run
//SKIP run-ir
//MOC-FLAG -A=M0270

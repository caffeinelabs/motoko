// A simple actor that checks if the subnet is an application subnet.
// This is done by running some instructions and burning some cycles
// and checking if the cycle balance is affected.

import Prim "mo:⛔";

actor  {

    public func test() : async () {
        let balance = Prim.cyclesBalance();

        var j : Nat = 0;
        while (j < 3) {
            j += 1;
            await async();
        };

        if (balance > Prim.cyclesBalance()) {
            Prim.debugPrint("Application subnet");
        };
    }
};

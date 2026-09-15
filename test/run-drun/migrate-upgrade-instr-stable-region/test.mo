//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:prim";
import Region "../stable-region/Region";

actor {
    stable var region = Region.new();
    ignore Region.grow(region, 1);
    Prim.debugPrint("Region size: " # debug_show(Region.size(region)));

    public func test() : async () {
        let instructions = Prim.rts_upgrade_instructions();
        // The classical→EOP migration is measured; the old module had no counter (sentinel Nat64 max).
        assert (instructions != 18_446_744_073_709_551_615);
        assert (instructions > 0);
        Prim.debugPrint("Ignore Diff: Upgrade instructions: " # debug_show (instructions));
    };
};

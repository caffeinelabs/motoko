import Prim "mo:prim";
import Region "../stable-region/Region";

actor {
    var region = Region.new();
    ignore Region.grow(region, 1);
    Prim.debugPrint("Region size: " # debug_show(Region.size(region)));

    public func test() : async () {
        Prim.debugPrint("Upgrade instructions: " # debug_show (Prim.rts_upgrade_instructions()));
    };
};

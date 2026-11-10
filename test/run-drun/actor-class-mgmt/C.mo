import Prim "mo:⛔";
import Cycles = "../cycles/cycles";
actor class C(n : Nat, contr : ?Principal, cycleStats : Bool) = this {
   var upgrades = 0;
   switch contr { case (?contr) assert Prim.isController contr; case _ () };

   system func preupgrade () {
     assert not Prim.isController(Prim.principalOfActor this);
     upgrades += 1;
     if cycleStats Prim.debugPrint ("Balance(preupgrade): " # debug_show Cycles.balance());
   };

   if cycleStats Prim.debugPrint ("Balance(init): " # debug_show Cycles.balance());

   public func observe () : async {args : Nat;  upgrades : Nat} {
     switch contr { case (?contr) assert Prim.isController contr; case _ () };
     { args = n; upgrades }
   };
}

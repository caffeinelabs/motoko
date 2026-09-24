actor {
   public shared query func f0() : async () { loop {} };
   let x0 = f0;

   public shared composite query func f1(_ : Nat, _ : Bool) : async (Nat, Bool) {
      loop {};
   };
   let x1 = f1;

   public shared func f2(_ : {#one; #two}, _ : { oldField : Int }) : async { oldField : Nat } {
      loop {};
   };
   let x2 = f2;
};

// Upgrade should fail (version 1 to 4).
actor {
   // Incompatible function type change:
   // - Return type is a sub-type (adding an object field).
   public shared func f2(_ : {#one}, _ : { oldField : Nat; newField: Text }) : async { oldField: Nat } {
      loop {};
   };
   let x2 = f2;
};

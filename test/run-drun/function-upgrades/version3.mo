// Upgrade should fail (version 1 to 3).
actor {
   // Incompatible function type change:
   // - Second parameter is a super-type of version 1 (removing a field).
   public shared func f2(_ : {#one}, _ : { }) : async { } {
      loop {};
   };
   let x2 = f2;
};

// reject uninitialized actor fields without --enhanced-migration
actor {

  let a : Nat;
  var b : Nat;

  transient let c : Nat;
  transient var d : Nat;

}

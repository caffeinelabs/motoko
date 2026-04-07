// reject uninitialized actor fields without --enhanced-migration
actor {

  stable let a : Nat;
  stable var b : Nat;

  transient let c : Nat;
  transient var d : Nat;

}

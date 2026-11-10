mixin() {
  let mixinNat : Nat;
  ignore mixinNat;

  transient let mixinTransient : Nat = 0;
  ignore mixinTransient;

  public shared func mixinFunc() : async () {};
}

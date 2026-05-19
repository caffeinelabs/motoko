// M12: mo_to_idl emits the refinement-pruned form of a GADT variant.
// `Expr<Bool>` at the wire should only carry the arms compatible with
// the instantiation: `#bool` (refinement A=Bool ✓) and `#if_` (no
// clause). The `#int` arm — refinement A=Nat ≠ Bool — is pruned.

actor {
  type Expr<A> = {
    #int  : type A = Nat in Nat;
    #bool : type A = Bool in A;
    #if_  : (Expr<Bool>, Expr<A>, Expr<A>);
  };

  public func receive(_ : Expr<Bool>) : async () {};
};

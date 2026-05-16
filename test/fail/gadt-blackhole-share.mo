// M12: a variant declaration carrying an existential ("black-hole")
// arm makes the whole type unshareable. Even an actor method that
// only deals with refinement-only arms via this type fails — the type
// itself is the unit of shareability, not the construction site.

type Expr<A> = {
  #int : type A = Nat in Nat;
  #bool : type A = Bool in A;
  #eq : type A = Bool, type B in ((B, B) -> Bool, Expr<B>, Expr<B>);  // black hole here
};

actor {
  // Should be rejected: Expr<Bool> carries a black-hole arm.
  public func receive(e : Expr<Bool>) : async () {};
};

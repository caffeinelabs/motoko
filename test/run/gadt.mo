type Expr<A> = {
  #int : type A = Nat in Nat;
  #bool : type A = Bool in A;
  #add : type A = Nat in (Expr<A>, Expr<A>);
  #if_                      : (Expr<Bool>, Expr<A>, Expr<A>);
  #eq : type A = Bool, type B in ((B, B) -> Bool, Expr<B>, Expr<B>);
};

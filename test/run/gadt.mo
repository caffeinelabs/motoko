type Expr<A> = {
  #int  type A = Nat        : Nat;
  #bool type A = Bool       : A;
  #add  type A = Nat        : (Expr<A>, Expr<A>);
  #if_                      : (Expr<Bool>, Expr<A>, Expr<A>);
  #eq   type A = Bool, type B : ((B, B) -> Bool, Expr<B>, Expr<B>);
};

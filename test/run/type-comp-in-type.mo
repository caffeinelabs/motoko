module M {
  public let value = null;
  public type T = Null;
  public type U<A> = A;
};

type MT = module {
  value : Null;
  type T = Null;
  type U<A> = A
};

ignore (M : module { value : Null; type T = Null; type U<A> = A });
ignore (M : MT);

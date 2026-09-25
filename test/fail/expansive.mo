do {
  type C<T> = ?C<T>; // accept
};

do {
  type C<T> = ?(T, C<T>); // accept
};

do {
  type C<T> = { #nil; #cons: (T, C<T>) }; // accept
};

do {
  type C<T> = { head:T; next: () -> C<T> }; // accept
};

do {
  type C<T, U> = ?C<U,T>; // accept
};

do {
  type C<T, U> = ?D<T,U>; // accept
  type D<T, U> = ?C<T,U>; // accept
};

do {
  type C<T, U> = ?D<U,T>; // accept
  type D<T, U> = ?C<U,T>; // accept
};

do {
  type C<T> = <A>C<T>->C<T>; // accept
};

do {
  type C<T> = ?C< <A>A->A >; // accept
};

ignore module {
  // accept
  public type C<T, U> = ?N.D<T,U>; // accept
  public module N = { public type D<T, U> = ?C<T,U>; }
};

do {
  type C<T> = { #nil; #cons: (T, C<{ #tag: T }>) }; // reject
};

do {
  type C<T> = { head:T; next: () -> C<{ field: T }> }; // reject
};

do {
  type C<T, U> = ?C<(T, T),U>; // reject
};

do {
  type C<T> = ?C<?T>; // reject
};

do {
  type C<T> = ?C< <A>T->T >; // reject
};

do {
  type C<T, U> = ?D<T,?U>; // reject
  type D<T, U> = ?C<T,U>;
};

do {
  type P<T> = Nat;
  do { type C<T> = ?P<C<C<T>>>; } // reject, but would accept after unfolding P (too conservative?)
};

do {
  type P<T> = Nat;
  do { type C<T> = ?C<P<T>>; } // reject, but would accept after unfolding P (too conservative?)
};

do {
  type P<T> = Nat;
  type C<T> = ?P<C<C<T>>>; // reject, but would accept after unfolding P (too conservative?)
};

ignore module {
  public type C<T, U> = ?N.D<T,?U>; // reject
  public module N = { public type D<T, U> = ?C<T,U>; }
};

do {
  type C<T> = <A <: C<T>>C<T>->C<T>; // accept
};

do {
  type C<T> = <A <: C<?T>>C<T>->C<T>; // reject, bad cycle in arrow bounds
};

do {
  type C<T <: C<T>> = ?C<T>; // accept
};

do {
  type C<T <: C<?T>> = ?C<T>; // reject, too conservative? Cycle is only in parameter bounds...
};

do {
  type C<T, U> = ?D<T,?T,Bool,Nat>; // accept
  type D<T, U, V, X> = ?C<T,U>;
};

do {
  type C<T, U> = ?D<T,?U,Bool,Nat>; // reject
  type D<T, U, V, X> = ?C<T,U>;
};

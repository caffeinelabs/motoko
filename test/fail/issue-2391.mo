// check class types are closed
func f<T>() {
  class C(x:T){ public let y = x }; // reject
  // ... because outer parameter T occurs in type of public field y
};

func g<T>() {
  class C<U>(x : T, u: U){ public let y = x; }; // reject
  // ... because outer parameter T occurs in type of public field y
};

func h<T1, T2>() {
  class C<U>(x1 : T1, x2: T2, u: U){ public let y = (x1, x2); }; // reject
  // ... because outer parameters T1, T2 occur in type of public field y
};

// test principal defaulting of underconstrained type parameters according to
// their variance in the result.
// TIP: best visually verified in VSCode

do {
  class Co<A>() {
    // covariant in A
    var item : ?A = null;
    public func get() : ?A{
      item
    }
  };

  let co = Co(); // accept, A covariant so lower bound `None` ok
  ignore co : Co<None>; // accept
  ignore co : Co<Any>; // accept
  let a : ? None = co.get(); // accept
  co; // reject, so we can see the type
};

do {
  class Contra<A>() {
    // contravariant in A
    var item : ?A = null;
    public func put(i : A) {
      item := ?i
    }
  };

  let contra = Contra(); // accept, A contravariant so upper bound `Any` ok
  ignore contra : Contra<Any>; // accept
  ignore contra : Contra<None>; // accept
  contra.put(10); // accept
  contra; // reject, so we can see the type
};

do {
  class Bound<A <: Nat>() {
    // contravariant in A
    var item : ?A = null;
    public func put(i : A) {
      item := ?i
    }
  };

  let bound = Bound(); // accept, A contravariant so upper bound `Nat` ok
  ignore bound : Bound<Any>; // reject
  ignore bound : Bound<Nat>; // accept
  ignore bound : Bound<None>; // accept
  bound.put(10); // accept
  bound; // reject, so we can see the type
};

do {
  class Non<A>() {
    // bivariant in A
    var item : ?Any = null;
    public func put(i : Any) {
      item := ?i
    };
    public func get() : ?Any{
      item
    }
  };
  let non = Non(); // accept, ambiguous but choice arbitrary since no occurrence of A in result.
  ignore non : Non<Any>; // accept
  ignore non : Non<None>; // accept
  non; // reject, so we can see the type
};

do {
  class Inv<A>() {
    var item : ?A = null;
    public func put(i : A) {
      item := ?i
    };
    public func get() : ?A{
      item
    }
  };

  let invNat1 : Inv<Nat> = Inv(); // acccept
  let invNat2 = Inv<Nat>(); // accept
  invNat1; // reject, so we can see the type
  invNat2; // reject, so we can see the type
  let _ = invNat1 : Inv<Any>; // reject due to invariance of Inv<_>.
  do {
    let inv = Inv(); // reject, A invariant and underconstrained
  }
};

do {
  func contraBi<T, U>(x : T, f : U -> ()) : T { x };
  ignore contraBi(1, func (x:Nat){}); // accept
};

do {
  func coBi<T, U>(x : T, f : () -> U) : T { x };
  ignore coBi(1, func () : Nat { 0 }); // accept
};

do {
  func invBi<T, U>(x : T, f : U -> U) : T { x };
  ignore invBi(1, func (x:Int) : Nat { 0 }); // accept
};

do {
  func invBi<T, U>(x : T, f : U -> (U, U)) : T { x };
  ignore invBi(1, func (x:{ #A; #B; #C }) : ({ #A }, { #B }) { (#A, #B) }); // accept
};

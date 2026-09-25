import { type T; type Result } "objpat-type-fields/lib";

func _result() {
  type MyNat = T;

  let _t : MyNat = 1;
  let _r1 : Result<Nat, Text> = #ok(1);
  let _r2 : Result<Nat> = #ok(3); // Wrong number of type parameters
};

func _bounds() {
  type Bounded<T <: Result<Int, Text>> = T;
  type BFine = Bounded<Result<Nat, Text>>;
  type BWrong = Bounded<Result<Float, Text>>;
};

type L1 = module { type N = Nat; Z : Nat };
type R1 = module { type N = Text };
type V1 = { #left : L1; #right : R1 };

func _f1(x : V1) {
  // Fine
  let (#left{ type N }) = x else {
    return ()
  };

  let _n : N = 10;
};

func _f1_1(x : V1) {
  // Error with mismatched types
  let (#left{ type N } or #right { type N }) = x else {
    return ()
  };
  let _n : N = 10;
};

func _f1_2(x : L1) {
  // Error with duplicate type field in pattern
  let { type N; type N } = x;
};

func _f1_3(x : L1) {
  // Error with missing type field in pattern
  let { type U } = x;
};

type L2 = module { type N = Nat };
type R2 = module { type N<A> = Nat };
type V2 = { #left : L2; #right : R2 };

func _f2(x : V2) {
  // Error with mismatched types
  let (#left{ type N } or #right { type N }) = x else {
    return ()
  };
  let _n : N = 10;
};

func _f2_1(x : V2) {
  // Error for different set of type bindings in patterns
  let (#left{ type N } or #right {}) = x else {
    return ()
  };
  let _n : N = 10;
};

type T3 = module { type N = Nat; x : module { type S = Text } };
func _f3(x : T3) {
  let ({ type N; x = { type S } }) = x;
  let t : N = 10;
  let s : S = "hi";
};

type T4 = (module { type N = Nat; }, Nat);
func _f4(x : T4) {
  let ({ type N }, _) = x;
  let _n : N = 10;
};

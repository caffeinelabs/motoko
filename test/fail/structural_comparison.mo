// Functions are not orderable
func m1() {
  (func (x : Nat) : Nat = x) < (func (x : Nat) : Nat = x);
};

// Mutable records are not orderable
func m2() {
  { var x = 10 } < { var x = 10 };
};

// Objects with non-orderable fields (local functions) are not orderable
func m3() {
  class A() { public func inner() : Nat = 1 };
  A() < A();
};

// Generic types are not orderable
func m4() {
  func myLt<A>(x : A, y : A) : Bool = x < y;
};

(func (x : Nat) : Nat = x) == (func (x : Nat) : Nat = x);

{ var x = 10 } == { var x = 10 };

class A() { public func inner() : Nat = 1 };
A() == A();

ignore ({ x = 10 } == { var x = 10 });

assert(10 == "hi");

func myEq<A>(x : A, y : A) : Bool = x == y;
func myEq2<A, B>(x : A, y : B) : Bool = x == y;

ignore (10 != "hi");
ignore ({ x = { p = 1 } } == { x = { q = 1 } });

// Exercise eq_con via object types with type aliases (type fields in obj expressions).
// eq_con is called in typing.ml:2947 for type field checks in object expressions with bases.
// It is also called in bi_match.ml:392 for type parameter matching.

// Object with base that has a type field — triggers eq_con in typing.ml
class Container() {
  public type Elem = Nat;
  public var count : Nat = 0;
  public func add() { count += 1 };
};

let c = Container();
c.add();
c.add();
assert c.count == 2;

// Generic function application that triggers bi_match's eq_con check
// on module type fields
module M {
  public type T = Nat;
  public let x : T = 42;
};

// Type fields in objects used with generics
type HasType = {type T = Nat; n : Nat};

// Trigger eq_binds via eq function on types with bound type params
// eq_binds is called in combine (lub/glb) for Func types
func poly<A, B>(a : A, b : B) : (A, B) = (a, b);
let _ = poly(1, "x");
let _ = poly(true, 0.5);

// Type alias chain that exercises Con(Def) vs Con(Abs) arm in eq_con'
type Alias1 = Nat;
type Alias2 = Alias1;
let _ : Alias2 = 42;

assert true;

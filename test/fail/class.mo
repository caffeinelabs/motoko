// Correct annotations
class B() : {x : Int} = {
  public let x : Nat = 0
};

type T<A, B> = {x : Int; f : A -> B};

class C<A>(y : Nat) : T<A, A> {
  public let x = y;
  public let z = 1;
  public func f(x : A) : A { x };
  public func g() {};
};

let c = C<Bool>(7);
let _ = c.x + c.z;
if (c.f(true)) { c.g() };

class D<A, B>() : C<B> {
  public let x = 0;
  public let z = 1;
  public func f(x : B) : B { x };
  public func g() {};
  public func h(x : A) {};
};


type A = actor {m : () -> ()};

actor class AA() : async A {
  public func m() : () {};
};


object o { public type T = Nat };
class E() : {a : o.T} = o {
  public let a = 0;
  public type T = Bool;
};


// Incorrect annotations
class D1() : {x : Int} {};
class D2() : {x : Nat} {
  public let x = -3;
};

class D3() : actor {} = {};
actor class D4() : actor {} = {};

object x { public type T = Nat };
class D5(x : {}) : {a : x.T} = {
  public let a = 0;
};

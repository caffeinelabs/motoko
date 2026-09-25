type Order = { #less; #greater; #equal };

func explicit1(n : Nat, m : Nat) : Order { #less };

module M {

  public func aardvark(n : Nat, m : Nat) : Order { #less };

  public func c(n : Nat, m: Nat) : Order {
    if n < m { #less }
    else if n == m { #greater }
    else { #equal };
  };

  public func ambiguous(n : Nat, m: Nat) : Order {
    if n < m { #less }
    else if n == m { #greater }
    else { #equal };
  };

};

module N {
  public func c(n : Text, m: Text) : Order {
    #equal;
  };

  public func ambiguous(n : Nat, m: Nat) : Order {
    if n < m { #less }
    else if n == m { #greater }
    else { #equal };
  };
};

func f1(n : Nat, m : Nat, c : (implicit : (Nat, Nat) -> Order)) {
  ignore c(n, m);
};

func f2<T>(n : T, m : T, c : (implicit : (T, T) -> Order)) {
  ignore c(n, m);
};

func f3(n : Nat, m : Nat, d : (implicit : Nat -> Order)) {};

f1(0, 1, M.c); //accept
f2(0, 1, M.c); //accept

f2("0", "1", N.c); // accept

f1(0, 1); //accept
f2(0, 1); //accept

f2("0", "1"); // accept

f2(true, true); // reject

f3(1, 1); // reject

func f4(n : Nat, m : Nat, bogus : (implicit : (Nat, Nat) -> Order)) {};

f4(1, 1); // reject

// retype f4 with as f4 with different implicit name
let f5 : (Nat, Nat, c : (implicit : (Nat, Nat) -> Order)) -> () = f4;

f5(1, 1); // accept

func f6(n : Nat, m : Nat, ambiguous : (implicit : (Nat, Nat) -> Order)) {};

f6(1, 1); // reject

module XY {
  public type XY = { x : Nat; y : Nat };
  public let zero : XY = { x = 0; y = 0 };
};

module XZ {
  public type XZ = { x : Nat; z : Nat };
  public let zero : XZ = { x = 0; z = 0 };
};

func mkZero<T>(zero : (implicit : T)) : T {
  zero
};

ignore mkZero<{ x : Nat }>();

// tricky case: if we have two implicit arguments of the same name we currently need to add a second type annotation
func c <T, U>(
  p1 : (T, U),
  p2 : (T, U),
  cT : (implicit : (c : (T, T) -> Order)),
  cU : (implicit : (c : (U, U) -> Order))
)
   : Order {
     switch cT(p1.0, p2.0) {
       case #equal { cU(p1.1, p2.1) }
       case ord { ord }
     };
   };

ignore c((1, "a"), (0, "b")); // accepted

func tuple(pair : (Nat, Nat), c : (implicit : (Nat, Nat) -> Order)) : Order {
  c(pair.0, pair.1)
};

ignore tuple((3, 3));

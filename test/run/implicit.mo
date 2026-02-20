//MOC-FLAG -A=M0194
import Prim "mo:prim";

type Order = {#less;#greater;#equal};

module M {

  public func c(n : Nat, m: Nat) : Order {
    Prim.debugPrint (debug_show (#c1(n,m)));
    if (n < m) #less
    else if (n == m) #greater
    else #equal;
  };
};

module N {
  public func c(n : Text, m: Text) : Order {
     Prim.debugPrint (debug_show (#c2(n,m)));
     #equal;
  };

};

func f1(n : Nat, m : Nat, c : (implicit : (Nat, Nat) -> Order)) {
  ignore c(n, m);
};

func f2<T>(n : T, m : T, c : (implicit : (T, T) -> Order)) {
  ignore c(n, m);
};

func f3(n : Nat, m : Nat, _ : (implicit : Nat -> Order)) {
};



f1(0, 1, M.c); //accept
f2(0, 1, M.c); //accept

//f1("0", "1", M.c); // reject
f2("0", "1", N.c); // accept

f1(0, 1); //accept
f2(0, 1); //accept

f2("0", "1"); // accept

do {

  func c(n : Text, m: Text) : Order {
     Prim.debugPrint (debug_show (#c3(n,m)));
     #equal;
  };

  f2("0", "1"); // accept (resolve to local c)
};

//f2(true, true); // reject

//f3(1, 1); // reject

func unary<T>(c : (implicit : (T, T) -> Order), t : T) {
  ignore c(t, t)
};

unary(10);

func nullary<T>(c : (implicit : (T, T) -> Order)) {
  ignore c;
};
nullary<Nat>();

module Nat {
  public func eq(x : Nat, y : Nat) : Bool {
    Prim.debugPrint("nat eq");
    x == y
  };
};

module Int {
  public func eq(x : Int, y : Int) : Bool {
    Prim.debugPrint("int eq");
    x == y
  };
};

func isEq<T>(x : T, y : T, eq : (implicit : (T, T) -> Bool)) : Bool {
  eq(x, y)
};

assert isEq<Nat>(3, 3);

func tuple(pair : (Nat, Nat), eq : (implicit : (Nat, Nat) -> Bool)) : Bool {
  eq(pair.0, pair.1)
};

assert tuple((3, 3));

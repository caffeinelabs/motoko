import Prim "mo:⛔";

type A = { #A };
type B = { #B };
type C = { #C };
func f0() {};
func f1<T>(x : T) : T { x };
func f2<T1, T2>(x1 : T1, x2 : T2) : (T1, T2) { (x1, x2) };
func f3<T1, T2, T3>(x1 : T1, x2 : T2, x3 : T3) : (T1, T2, T3) { (x1, x2, x3) };

() |> f0 _;

() |> f0 (_);

let (#A, #B) = #A |> f2(_, #B);
let (#A, #B) = #B |> f2(#A, _);
let (#A, #B, #C) = #A |> f3(_, #B, #C);
let (#A, #B, #C) = #B |> f3(#A, _, #C);
let (#A, #B, #C) = #C |> f3(#A, #B, _);

/* left associative nesting */

let ((#A, #B), #C) = #A |> f2(_, #B) |> f2(_, #C);
let (#C, (#A, #B)) = #A |> f2(_, #B) |> f2(#C, _);

let ((#A, #B), #C) = (#A |> f2(_, #B)) |> f2(_, #C);
let (#C, (#A, #B)) = (#A |> f2(_, #B)) |> f2(#C, _);

/* eval order */

func A() : A { Prim.debugPrint(debug_show(#A)); #A };
func B() : B { Prim.debugPrint(debug_show(#B)); #B };
func C() : C { Prim.debugPrint(debug_show(#C)); #C };

func f<F>(f : F) : F {
  Prim.debugPrint("f");
  f
};

Prim.debugPrint("1:");
let (#A, #B) = A() |> (f f2)(_, B());
Prim.debugPrint("2:");
let (#A, #B) = B() |> (f f2)(A(), _);
Prim.debugPrint("3:");
let (#A, #B, #C) = A() |> (f f3)(_, B(), C());
Prim.debugPrint("4:");
let (#A, #B, #C) = B() |> (f f3)(A(), _, C());
Prim.debugPrint("5:");
let (#A, #B, #C) = C() |> (f f3)(A(), B(), _);

/* contrived example */

type Iter<T> = { next : () -> ?T };

func sum(ns : Iter<Nat32>) : Nat32 {
  var sum : Nat32 = 0;
  for n in ns { sum += n };
  sum
};

func map<A, B>(xs : Iter<A>, f : A -> B) : Iter<B> = object {
  public func next() : ?B {
    switch xs.next() {
      case ?next {
        ?f(next)
      }
      case null {
        null
      }
    }
  }
};

let 532 = "hello".chars() |> map(_, Prim.charToNat32) |> sum _;

let 532 = Prim.charToNat32 |> map("hello".chars(), _) |> sum (_);

/* eval order, continued */

do {
  // check eval order preserved, even for piped lvalues
  var i = 0;
  func f() : Nat -> Nat {
    i := 1;
    func (j : Nat) : Nat { j };
  };

  // should read i before executing f()
  assert (i |> (f())(_) == 0);
};

/* option blocks */

let _ = do ? {
  let none = null;
  (none |> f1(_)) !
};

let _ = do ? {
  let some = ??();
  (some ! |> f1(_))!;
};

/* non-linear, free-form piping */

let five = 2 |> _ * _ |> _ + 1;

assert (five == 5);

import Prim "mo:prim";

func mapMono<T>(ar : [T], _f : T -> T) : [T] = ar;
func mapMonoTuple<T>(ar : [T], _t : (T -> T, T -> T)) : [T] = ar;
func filter<T>(array : [T], _f : T -> Bool) : [T] = array;
func filterTuple<T>(ar : [T], _t : (T -> Bool, T -> Bool)) : [T] = ar;

func forEach<T>(_ar : [T], _f : T -> ()) {};

func map<I, O>(_ar : [I], _f : I -> O) : [O] = [];
type Alt<A, B> = { #inj1 : A; #inj2 : B };
func mapAlt<I, O1, O2>(_ar : [I], _f : I -> Alt<O1, O2>) : [Alt<O1, O2>] = [];

let ar = [1, 2, 3];

// Use `ar` to infer all type variables (just T), check lambda after
let _ = mapMono(ar, func x = x + 1);
let _ = mapMonoTuple(ar, (func x = x + 1, func x = x + 1));
let _ = filter(ar, func x = x > 1);
let _ = filterTuple(ar, (func x = x > 1, func x = x > 1));
forEach(ar, func x = ());

// Infer I=Nat from ar, then infer O from the body
let _ = map(ar, func x = x + 1); // O=Nat
let _ = map(ar, func x = debug_show x # "!"); // O=Text

// Check that optional return type can be provided
let _ = map(ar, func x : Int = x + 1); // func return type annotation
let _ = map(ar, func x = x + 1 : Int); // body type annotation
let _ : [Int] = map(ar, func x = x + 1); // result type annotation

// Unmentioned type variables are inferred too
let _ = mapAlt(ar, func x = #inj1 x); // O=Alt<Nat, None>
let _ = mapAlt(ar, func x = #inj2 x); // O=Alt<None, Nat>

func fail<T>() : T = Prim.trap("fail");

type Compare<T> = (T, T) -> { #less; #equal; #greater };

type Foo1<T> = { var x : T };
type Foo2<T> = { x : T };

func foo1<T, R>(_ : Foo1<T>, _ : Compare<R>, _ : T -> R) : Foo1<R> = fail();
func foo2<T, R>(_ : Foo2<T>, _ : Compare<R>, _ : T -> R) : Foo2<R> = fail();

let f1 : Foo1<Nat> = { var x = 1 };
let f2 : Foo2<Nat> = { x = 1 };

func textCompare(_a : Text, _b : Text) : { #less; #equal; #greater } = #equal;
func natToText(_n : Nat) : Text = "";

let _ : Foo1<Text> = foo1(f1, textCompare, func x = natToText(x) # "!");
let _ = foo2(f2, textCompare, func x = natToText(x) # "!");

// Only fix A in the 1st round
func foo<T, A>(t : T, _ : A, _ : A -> T) : T = t;
// In the 1st round: Nat <: T  (don't fix T yet!)
// In the 2nd round: Int <: T  (solve T := Int)
let _ = foo(1, "abc", func _ = -1);

module ClosedBody1 {
  func f1<A>(_ : A, _ : A -> Int) {};
  func f2<A, B>(_ : A, _ : A -> Int) {};
  // It should defer the func, solve A=Nat in the 1st round, but (1 : Int) should be checked and leave nothing to solve in the 2nd round
  func _main() {
    f1(1, func _ = 1);
    f2(1, func _ = 1); // extra unused type variable B
  };
};

module ClosedBody2 {
  func f1<A, B>(a : A, f : A -> B) : B = f(a);
  func f2<A, B, C>(a : A, f : A -> B) : B = f(a);
  // Like above, but here we have the return type annotation, it should check the body (1 : Int) and add (Int <: B) to solve in the 2nd round
  func _main() {
    let _ = f1(1, func _ : Int = 1);
    let _ = f2(1, func _ : Int = 1); // extra unused type variable C
  };
};

module MustSolveComplex {
  type Arg<A, B> = { var x : [({ #v1 : A }, B)] };
  type In1<A> = [?A];
  type In2<B> = { #v2 : [var B] };
  type In<A, B> = (In1<A>, In2<B>);
  func f1<A, B, O>(_ : Arg<A, B>, _ : In<A, B> -> O) : O = fail();
  func f2<A, B, O>(_ : Arg<A, B>, _ : (In1<A>, In2<B>) -> O) : O = fail();
  func _main() {
    let a = { var x = [(#v1(1), 0xf)] };
    let _ = f1(a, func(x, y) = (x[0], switch y { case #v2(y) { y } }));
    let _ = f2(a, func p = (p.0 [0], switch p.1 { case #v2(y) { y } }));
  };
};
//SKIP comp
//SKIP run
//SKIP run-drun
//SKIP run-ir
//SKIP run-low

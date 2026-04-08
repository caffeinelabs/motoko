//MOC-FLAG -A=M0194
import Prim "mo:⛔";

// Branch warnings.

let _ = if true true else 5;
let _ = if true true else [var 5];
let _ = if true 5 else 5.1;
let _ = if true {} else ({});
let _ = if true ({x = 5}) else ({var x = 5});
let _ = if true 1 else (-1);  // ok
let _ = if true true else (5 : Any);  // ok

let _ = switch 0 { case 0 true; case _ 5 };
let _ = switch 0 { case 0 true; case _ [var 5] };
let _ = switch 0 { case 0 2; case 2 (-5); case 3 "text"; case _ () };
let _ = switch 0 { case 0 true; case _ (() : Any); };  // ok


// Array warnings.

let _ = [true, 5];
let _ = [true, [var 5]];


// Local types (not) escaping.

let x =
do {
  type T = {};
  let o : T = {};
  o;
};

let y =
do {
  class C() {};
  type T<A> = Int;
  let n : T<C> = 5;
  n;
};

type T<A> = Int;
let z =
do {
  class C() {};
  let n : T<C> = 5;
  n;
};

class C() {};
let a =
do {
  let n : T<C> = 5;
  n;
};


// Any Type

type Top = Any;
func top(top : Top) {
  let _ = [1, top];
  let _ = [true, top];
  let _ = [null, top];
  let _ = ["", top];
  let _ = [top, top];
};


// Bottom Type

do {
type Bot = None;
func bot(bot : Bot) {
  //let a = bot.1;
  let b = bot.x;
  let c = bot();
  let d = bot(1, 2);
  let e = bot<Int>(5);
  let f = bot[1];
  let g = bot + bot * bot;
  for (x in bot) {};
};
};

// This was an error when classes were subtypes of their
// representation, but with structural typing throughout it is
// no longer one
do {
  let _ =
  do {
    class C() {};
    type T = C;
    let o : T = C();
    o;
  };
};


// Function types.

func f(h : Int -> Int) : Int = h 5;
func g(h : (Int, Int) -> Int) : Int = h(5, 6);
func k(h : {#A; #B : Int} -> Int) : Int = h(#B 9);

let _ = f(func x = x);
let _ = f(func x = x + 1);
let _ = f(func x = Prim.abs(x + 1));
let _ = f(func x = -1);
let _ = g(func p = Prim.abs(p.0));
let _ = g(func(x, y) = x + y);
let _ = g(func(x, y) = Prim.abs x);
let _ = k(func(#A or #B _) = 0);

let _ = f(func x : Int = x);
let _ = f(func x : Int = x + 1);
let _ = f(func x : Nat = Prim.abs(x + 1));
let _ = f(func x : Int = 0);
let _ = g(func p : Nat = Prim.abs(p.0));
let _ = g(func(x, y) : Int = x + y);
let _ = g(func(x, _) : Nat = Prim.abs x);
let _ = k(func(#A or #B _) : Nat = 0);

let _ = f(func(x : Int) : Int = x);
let _ = f(func(x : Int) : Int = x + 1);
let _ = f(func(x : Int) : Nat = Prim.abs(x + 1));
let _ = f(func(x : Any) : Int = 0);
let _ = g(func(p : (Int, Any)) : Nat = Prim.abs(p.0));
let _ = g(func(x : Int, y : Int) : Int = x + y);
let _ = g(func(x : Int, _ : Any) : Nat = Prim.abs x);
let _ = g(func((x, _) : (Int, Any)) : Nat = Prim.abs x);
let _ = k(func(#A or #B (_ : Any)) : Nat = 0);
let _ = k(func((#A or #B _) : {#A; #B : Any}) : Nat = 0);

let _ = f(func<>(x : Int) : Int = x);
let _ = f(func<>(x : Int) : Int = x + 1);
let _ = f(func<>(x : Int) : Nat = Prim.abs(x + 1));
let _ = f(func<>(x : Any) : Int = 0);
let _ = g(func<>(p : (Int, Any)) : Nat = Prim.abs(p.0));
let _ = g(func<>(x : Int, y : Int) : Int = x + y);
let _ = g(func<>(x : Int, y : Any) : Nat = Prim.abs x);
let _ = g(func<>((x, _) : (Int, Any)) : Nat = Prim.abs x);
let _ = k(func<>(#A or #B (_ : Any)) : Nat = 0);
let _ = k(func<>((#A or #B _) : {#A; #B : Any}) : Nat = 0);


// Subtraction warnings

func sub() {
  let n : Nat = 0;
  let i : Int = 0;
  func fn() : Nat { 0 };
  func fi() : Int { 0 };

  let _ = 2 * (n - 1);     // warn
  let _ = n * (n - 1);     // warn
  let _ = i * (n - 1);     // warn
  let _ = fn() * (n - 1);  // warn
  let _ = fi() * (n - 1);  // warn
  let _ = 2 * (i - 1);     // don't warn

  let _ : Nat = 2 * (n - 1);     // don't warn
  let _ : Nat = n * (n - 1);     // don't warn
  let _ : Nat = fn() * (n - 1);  // don't warn
  let _ : Int = 2 * (n - 1);     // don't warn
  let _ : Int = n * (n - 1);     // don't warn
  let _ : Int = i * (n - 1);     // don't warn
  let _ : Int = fn() * (n - 1);  // don't warn
  let _ : Int = fi() * (n - 1);  // don't warn

  let _ = 2 == n - 1;     // warn
  let _ = n == n - 1;     // warn
  let _ = i == n - 1;     // warn
  let _ = fn() == n - 1;  // warn
  let _ = fi() == n - 1;  // warn
  let _ = 2 == i - 1;     // don't warn

  let a = [1, 2];
  let _ = a[n - 1];  // don't warn

  func f(n : Nat) {};
  func g<T>(x : T) {};

  f(n - 1);       // don't warn
  g<Nat>(n - 1);  // don't warn
  g(n - 1);       // warn

  func h() : Nat { n - 1 };  // don't warn
};

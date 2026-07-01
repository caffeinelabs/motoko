// Exercise `shift` and `subst` via generic functions whose type contains
// Func, Array, Tup, Opt, Variant, Obj, Mut, Any, Non shapes —
// all of which appear in the `subst` traversal (close/open_ calls in typing.ml).

// Generic function returning a Func type — subst encounters Func, calls shift
func compose<A, B, C>(f : B -> C, g : A -> B) : A -> C =
  func(x : A) : C = f(g(x));

let inc : Nat -> Nat = func(n : Nat) : Nat = n + 1;
let dbl : Nat -> Nat = func(n : Nat) : Nat = n * 2;
let incDbl = compose<Nat, Nat, Nat>(dbl, inc);  // dbl(inc(n))
assert incDbl(3) == 8;

// Generic function with Opt return type — subst encounters Opt
func liftOpt<T>(x : T) : ?T = ?x;
assert (liftOpt(42) == ?42);
assert (liftOpt("hi") == ?"hi");

// Generic function returning Array — subst encounters Array
func singleton<T>(x : T) : [T] = [x];
assert (singleton(7).size() == 1);
assert (singleton("a")[0] == "a");

// Generic function with Tup — subst encounters Tup
func makePair<A, B>(a : A, b : B) : (A, B) = (a, b);
assert (makePair(1, "x") == (1, "x"));
assert (makePair(true, 0.5) == (true, 0.5));

// Generic function with Variant — subst encounters Variant
type Result<T> = {#ok : T; #err : Text};
func ok<T>(x : T) : Result<T> = #ok x;
func err<T>(msg : Text) : Result<T> = #err msg;
assert (ok(42) == (#ok 42 : Result<Nat>));

// Generic function with Obj — subst encounters Obj
func box<T>(v : T) : {value : T} = {value = v};
assert (box(99).value == 99);
assert (box("z").value == "z");

// Higher-order generic — nested Func in Func type body
func twice<T>(f : T -> T, x : T) : T = f(f(x));
assert (twice(inc, 3) == 5);
assert (twice(func(s : Text) : Text = s # s, "ab") == "abab");

// Generic with multiple type params appearing in Func body
func mapPair<A, B>(f : A -> B, p : (A, A)) : (B, B) =
  (f(p.0), f(p.1));
assert (mapPair(inc, (2, 3)) == (3, 4));

// Mut in generic — subst encounters Mut
func mutBox<T>(init : T) : {var value : T} =
  {var value = init};
let mb = mutBox<Nat>(5);
mb.value := 10;
assert mb.value == 10;

// Array subst with higher-kind: Array inside Func
// Nested generics — deeper close/shift chain
func flip<A, B>(f : A -> B -> A) : B -> A -> A =
  func(b : B) : A -> A = func(a : A) : A = f(a)(b);
let f1 = flip<Nat, Nat>(func(n : Nat) : Nat -> Nat = func(m : Nat) : Nat = n + m);
assert (f1(10)(3) == 13);

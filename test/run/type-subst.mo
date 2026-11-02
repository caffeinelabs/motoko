// shift / subst / close / open_ over every type shape: via generic function
// instantiation, then via generic type application.

func compose<A, B, C>(f : B -> C, g : A -> B) : A -> C = func(x : A) : C = f(g(x));
let inc : Nat -> Nat = func(n : Nat) : Nat = n + 1;
let dbl : Nat -> Nat = func(n : Nat) : Nat = n * 2;
assert (compose<Nat, Nat, Nat>(dbl, inc)(3) == 8);

func liftOpt<T>(x : T) : ?T = ?x;
assert (liftOpt(42) == ?42);
assert (liftOpt("hi") == ?"hi");

func singleton<T>(x : T) : [T] = [x];
assert (singleton(7).size() == 1);
assert (singleton("a")[0] == "a");

func makePair<A, B>(a : A, b : B) : (A, B) = (a, b);
assert (makePair(1, "x") == (1, "x"));
assert (makePair(true, 0.5) == (true, 0.5));

type Result<T> = {#ok : T; #err : Text};
func ok<T>(x : T) : Result<T> = #ok x;
assert (ok(42) == (#ok 42 : Result<Nat>));

func box<T>(v : T) : {value : T} = {value = v};
assert (box(99).value == 99);

func twice<T>(f : T -> T, x : T) : T = f(f(x));
assert (twice(inc, 3) == 5);
assert (twice(func(s : Text) : Text = s # s, "ab") == "abababab");

func mapPair<A, B>(f : A -> B, p : (A, A)) : (B, B) = (f(p.0), f(p.1));
assert (mapPair(inc, (2, 3)) == (3, 4));

func mutBox<T>(init : T) : {var value : T} = {var value = init};
let mb0 = mutBox<Nat>(5);
mb0.value := 10;
assert (mb0.value == 10);

func flip<A, B>(f : A -> B -> A) : B -> A -> A =
  func(b : B) : A -> A = func(a : A) : A = f(a)(b);
assert (flip<Nat, Nat>(func(n : Nat) : Nat -> Nat = func(m : Nat) : Nat = n + m)(10)(3) == 13);

type ArrayOf<T> = [T];
let _a1 : ArrayOf<Nat> = [1, 2, 3];
let _a2 : ArrayOf<Text> = ["a", "b"];
type Pair<A, B> = (A, B);
let _p1 : Pair<Nat, Bool> = (42, true);
let _p2 : Pair<Text, Int> = ("hi", -1);
type Maybe<T> = ?T;
let _m1 : Maybe<Nat> = ?42;
type Either<A, B> = {#left : A; #right : B};
let _l : Either<Nat, Text> = #left 42;
let _r : Either<Nat, Text> = #right "hello";
type Box<T> = {value : T};
let _bx : Box<Nat> = {value = 7};
type MutBox<T> = {var value : T};
let mb : MutBox<Nat> = {var value = 0};
mb.value := 42;
type Transform<A, B> = A -> B;
let tf : Transform<Nat, Bool> = func(n : Nat) : Bool = n > 0;
assert (tf(5));
type NestedOpt<T> = ?(T, ?T);
let _no : NestedOpt<Nat> = ?(1, ?2);
type Lst<T> = ?(T, Lst<T>);
let _list : Lst<Nat> = ?(1, ?(2, null));
type PairArr<T> = ([T], [T]);
let _paa : PairArr<Nat> = ([1, 2], [3, 4]);
type Nested<A, B> = {fst : A; snd : B; both : Pair<A, B>};
let _ne : Nested<Nat, Text> = {fst = 1; snd = "x"; both = (1, "x")};
assert (mb.value == 42);
assert (_l == #left 42);
assert (_r == #right "hello");

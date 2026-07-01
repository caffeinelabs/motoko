// Exercise the `shift` function in type.ml by constructing generic types
// that require type variable index adjustment during open_/reduce.
// shift is called when substituting type variables into:
//   Array, Tup, Func, Opt, Async, Obj, Variant, Mut, Any, Non, Named, Weak
// This happens when instantiating polymorphic type constructors.

// Array t -> Array (shift i n t): array of type variable
type ArrayOf<T> = [T];
let _ : ArrayOf<Nat> = [1, 2, 3];
let _ : ArrayOf<Text> = ["a", "b"];

// Tup ts -> Tup (List.map (shift i n) ts): tuple of type vars
type Pair<A, B> = (A, B);
let _ : Pair<Nat, Bool> = (42, true);
let _ : Pair<Text, Int> = ("hi", -1);

// Opt t -> Opt (shift i n t): optional type variable
type Maybe<T> = ?T;
let _ : Maybe<Nat> = ?42;
let _ : Maybe<Text> = ?"hello";

// Variant fs -> Variant (List.map (shift_field n i) fs): variant with type var
type Either<A, B> = {#left : A; #right : B};
let _l : Either<Nat, Text> = #left 42;
let _r : Either<Nat, Text> = #right "hello";

// Obj: object with type variable fields
type Box<T> = {value : T};
let _ : Box<Nat> = {value = 7};
let _ : Box<Bool> = {value = true};

// Mut t -> Mut (shift i n t): mutable field with type variable
type MutBox<T> = {var value : T};
let mb : MutBox<Nat> = {var value = 0};
mb.value := 42;

// Func (s, c, tbs, ts1, ts2): generic function type with nested type vars
type Transform<A, B> = A -> B;
let f : Transform<Nat, Bool> = func(n : Nat) : Bool = n > 0;
assert f(5);

// Nested generics — deeper substitution chain
type NestedOpt<T> = ?(T, ?T);
let _ : NestedOpt<Nat> = ?(1, ?2);

// Con (c, ts): applying a type con to type args — triggers Con branch in shift
type List<T> = ?(T, List<T>);
let _list : List<Nat> = ?(1, ?(2, null));

// Pair of arrays — exercises both Array and Tup in shift
type PairArr<T> = ([T], [T]);
let _ : PairArr<Nat> = ([1, 2], [3, 4]);

// Nested polymorphic definitions that trigger Var(j < i) branch
// When j < i in shift, j is returned unchanged — this happens for bound vars
// in deeply nested generic bodies.
type Nested<A, B> = {fst : A; snd : B; both : Pair<A, B>};
let _ : Nested<Nat, Text> = {fst = 1; snd = "x"; both = (1, "x")};

assert (mb.value == 42);
assert (_l == #left 42);
assert (_r == #right "hello");

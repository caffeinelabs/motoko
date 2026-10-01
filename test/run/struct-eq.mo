//MOC-FLAG -A=M0194 -W M0276
assert({ x = 10 } == { x = 10 });
assert({ x = 10 } != { x = 9 });

assert(?10 == ?10);
assert(null == null);
assert(?10 != null);

assert(#x == #x);
assert(#ok(10) == #ok(10));
assert((#x : { #x; #y }) != #y);
assert(#ok(10) != #ok(9));

assert({ x = 10; y = 9 } == { x = 10 });
assert([1, 2, 3] == [1, 2, 3]);
assert([1, 2, 3] != [1, 2, 3, 4]);
assert([?10] == [?10]);
assert([?10] != [null]);

type List<A> = ?(A, List<A>);

let xs : List<Nat> = ?(10, ?(20, null));
let ys : List<Nat> = ?(20, ?(10, null));
assert(xs != ys);

type T = ((), T);
func e(t1 : T, t2 : T) : Bool = t1 == t2;

// compared at a common type with content, only a warning
assert({ id = 1; name = "a" } != { id = 2; email = "b" });
assert((1, { p = 1 }) == (1, { q = 2 }));
assert((null : ?{ p : Nat }) != (?{ q = 1 } : ?{ q : Nat }));
assert((#a : { #a; #b }) != (#c : { #a; #c }));
func f<A <: { id : Nat }>(a : A, b : A) : Bool = a == b;
assert(not f({ id = 1; name = "a" }, { id = 2; name = "a" }));
func g(a : actor { f : () -> async () }, b : actor { g : () -> async () }) : Bool = a == b;

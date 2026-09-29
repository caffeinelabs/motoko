//MOC-FLAG -A=M0194
// comparisons at a common type that still has observable content only warn M0062
func f1(a : { id : Nat; name : Text }, b : { id : Nat; email : Text }) : Bool = a == b;
func f2(a : (Nat, { p : Nat }), b : (Nat, { q : Nat })) : Bool = a == b;
func f3(a : actor { f : () -> async () }, b : actor { g : () -> async () }) : Bool = a == b;
func f4(a : ?{ p : Nat }, b : ?{ q : Nat }) : Bool = a == b;
func f5(a : {#a : { p : Nat }; #b}, b : {#a : { q : Nat }; #c}) : Bool = a != b;
func f6<A <: { id : Nat }>(a : A, b : A) : Bool = a == b;

assert f1({ id = 1; name = "a" }, { id = 1; email = "b" });
assert not f1({ id = 1; name = "a" }, { id = 2; email = "b" });
assert not f2((1, { p = 1 }), (2, { q = 1 }));
assert not f4(null, ?{ q = 1 });
assert f5(#b, #c);
assert not f6({ id = 1; name = "a" }, { id = 2; name = "a" });

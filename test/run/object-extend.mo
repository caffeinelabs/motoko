//MOC-FLAG -A=M0194
//MOC-FLAG --experimental-field-aliasing
import Prim "mo:⛔";

// synthesis
let b = { b = 6 };
module m { public let b = 6 };
Prim.debugPrint (debug_show { b with a = 8 });
Prim.debugPrint (debug_show { b and m with b = 8 });
Prim.debugPrint (debug_show { { b = 6; c = "C" } with a = 8 });
Prim.debugPrint (debug_show { { c = 'C'; d = "D" } with a = 8; b = 6 });

// analysis
ignore ({ b with a = 8 } : { a : Nat });
ignore ({ b with a = 8 } : { a : Nat; b : Nat });
ignore ({ b and m with a = 8 : Int; b = 'X'} : { a : Int; b : Char });
ignore ({ b and m and m with a = 8 : Int; b = 'X' } : { a : Int; b : Char });
ignore ({ b and m and m and b with a = 8 : Int; b = 'X' } : { a : Int; b : Char });

// var fields
let c = { var c = 25 };

let d = { c with var c = c.c };
c.c += 1;
assert c.c == d.c + 1;

let e = { c with e = 42 };
c.c += 1;
assert c.c == e.c;

// methods closing over var fields
let c0 = object {
    public var c : Int = 0;
    public func incr() = c += 1
};

let c1 = { c0 with d = 2; };

assert c1.c == 0;
c1.incr();
assert c1.c == 1;
assert c0.c == 1;

// this is checking that the interpreter doesn't consider
// dynamic fields not present in the static type

let r : {} = { l = 0 };
let s = { l = true };
let t = { r and s };
assert t.l;

// some allowed polymorphism

type A0 = { a : Int };
type B0 = { b : Char };

func mix<A <: A0, B <: B0>(a : A, b : B) : A0 and B0 and { c : Text } =
    { a and b with c = "Waay to go!" };

func mox<A <: { a : Int }, B <: { b : Char }>(a : A, b : B) : { a : Int } and { b : Char } and { c : Text } =
    { a and b with c = "Right" };

func mux<A <: { a : Int }, B <: { b : Char }>(a : A, b : B) : { a : Int; b : Char; c : Text } =
    { a and b with c = "Yeah" };


// extending iterators
let tb_ok : { next : () -> ?Char; bar : Nat } = { "Text base".chars() with bar = 42 };
let ab_ok : { next : () -> ?Text; bar : Nat } = { ["Array base"].values() with bar = 42 };

// detecting lost fields
let ab_ok_warn : { next : () -> ?Text; bar : Nat } = { ab_ok with bur = 42 };
let foo_warn : { foo : Text } = { foo = "FOO"; quux = "QUUX" };
let bar_warn : { foo : Text } = object { public let foo = "FOO"; public let quux = "QUUX" };
object obj_warn : { foo : Text } { public let foo = "FOO"; public let quux = "QUUX" };
class() : { foo : Text } = { public let (foo, quux) = ("FOO", "QUUX") };

// don't confuse with types
let bar_type_warn_type : { foo : Text } = object { public let foo = "FOO"; public type Quux = Text };

let bar_type_warn : { type Quux = Text; foo : Text } = object { public let foo = "FOO"; public let Quux = "QUUX"; public type Quux = Text };

object obj_warn_type : { foo : Text } { public let foo = "FOO"; public type Quux = Text };
class() : { foo : Text } = { public let foo = "FOO"; public type Quux = Text };

//MOC-FLAG -A=M0194
// Top Type

do {
type A = {x : Int; y : Int};
class C() {};
func f1(x : Any) : Any = x;
func f2(x : A) : Any = x;
func f3(x : C) : Any = x;
func f4(x : Null) : Any = x;
func f5(x : Int) : Any = x;
};

// Bottom Type

do {
type A = {x : Int; y : Int};
class C() {};
func f1(x : None) : None = x;
func f2(x : None) : A = x;
func f3(x : None) : C = x;
func f4(x : None) : Null = x;
func f5(x : None) : Int = x;
};

// Number Types

do {
func f(x : Nat) : Int = x;
};

// Option Type

do {
class C() {};
func f1(x : Null) : ?C = x;
// func f2(x : C) : ?C = x;
func f3(x : ?Nat) : ?Int = x;
// func f4(x : Nat) : ?Int = x;
func f5(x : Null) : ?Null = x;
// func f6(x : ?C) : ??C = x;
};

// Object Types

do {
type A = {x : Int; y : Int};
type B = {x : Int};
func f(x : A) : B = x;
};

do {
type A = {x : Int; y : Bool; z : Nat};
type B = {y : Bool; x : Int};
func f(x : A) : B = x;
};

do {
type A = {x : Int; y : Bool};
type B = {y : Bool; x : Int; z : Nat};
type C = {y : Bool; x : B};
type D = {x : {x : Int; y : Bool}; y : Bool};
func f(x : C) : D = x;
};

// Function Types

do {
type A = (Int, Bool) -> (Nat8, Nat);
type B = (Nat, Bool) -> (Nat8, Int);
func f(x : A) : B = x;
};

do {
type A = {x : Int} -> {x : Any} -> {r : Nat; z : Bool};
type B = {y : Float; x : Nat} -> {x : Bool} -> {r : Int};
func f(x : A) : B = x;
};


// Type Abbreviations

do {
type T<X, Y> = Y;
type A = T<Bool, Nat>;
type B = T<Int, Int>;
func f(x : A) : B = x;
};


// Classes

do {
class C<X>() {public func f(x : X) {}};
type A = C<Int>;
type B<X> = {f : X -> ()};
func f(x : A) : B<Int> = x;
};

do {
class C<X>() {public func f(x : X) {}};
type A = C<Int>;
type B<X> = {f : X -> ()};
func f(x : A) : B<Nat> = x;
};


// Bounds

do {
type A = <X> X -> X;
type B = <Y> Y -> Y;
func f(x : A) : B = x;
};

do {
type A = <X <: Int, Y <: {}> X -> Y;
type B = <X <: Int, Y <: {}> X -> Y;
func f(x : A) : B = x;
};

do {
type A = <X <: Nat, Y> X -> Y;
type B = <Y <: Nat, X> Y -> X;
func f(x : A) : B = x;
};

do {
type A = <X, Y> X -> X;
type B = <X, Y> X -> X;
func f(x : A) : B = x;
};

do {
type A = <X, Y, Z <: X> X -> X;
type B = <X, Y, Z <: X> X -> X;
func f(x : A) : B = x;
};

do {
class C() { public let x = 0 };
type A = <X <: C> X -> X;
type B = <X <: C> X -> X;
func f(x : A) : B = x;
};

do {
class C<X <: Int>() {public func f() : X { f() }};
type A = <X <: Int, Y <: {f : () -> X}> X -> X;
type B = <X <: Int, Y <: C<X>> X -> X;
func f(x : A) : B = x;
};

do {
class C<X <: Int>() {public func f() : X { f() }};
type A = <X, Y <: {f : () -> Nat}> X -> X;
type B = <X, Y <: C<Nat>> X -> X;
func f(x : A) : B = x;
};


// Recursion

/* TBR: Should this work? It's fine coinductively.
do {
type A = A;
type B = B;
func f(x : A) : A = x : B;
};
*/

do {
type A = {x : A; y : A};
type B = {x : B};
func f(x : A) : B = x;
};

do {
type A0 = {x : A0};
type A = {x : A0};
type B = {x : B};
func f(x : A) : B = x;
};

do {
type A1 = {x : A2};
type A2 = {x : A1};
type B = {x : B};
func f(x : A1) : B = x;
func g(x : A2) : B = x;
};

do {
type A1 = {x : A2; y : Int; z : Bool};
type A2 = {x : A1; y : Int};
type B1 = {x : B2; y : Int};
type B2 = {x : B1};
func f1(x : A1) : A1 = x;
func f2(x : A2) : A2 = x;
func g1(x : A1) : B1 = x;
func g2(x : A1) : B2 = x;
func h1(x : A2) : B1 = x;
func h2(x : A2) : B2 = x;
};

do {
type A = {x : {x : A; y : Int}; z : Nat};
type B = {x : B};
func f(x : A) : B = x;
};

do {
type A = {x : {x : {x : A}; y : Nat}};
type B = {x : {x : B}};
func f(x : A) : B = x;
};

do {
type A1 = {x : A2; y : B1};
type A2 = {x : A3; y : Bool};
type A3 = {x : A1; y : {}};
type B1 = {x : B2};
type B2 = {x : B1};
func f(x : A1) : B1 = x;
func g(x : A2) : B1 = x;
func h(x : A3) : B1 = x;
};

do {
type A1<T> = {x : A2<T>; y : T; z : {}};
type A2<T> = {x : A1<T>; z : Nat; y : Int};
type B1<T> = {x : C2<T>; y : T};
type B2<T> = {x : C1<T>; z : Nat};
type C1<T> = {x : B2<T>; y : T};
type C2<T> = {x : B1<T>; z : Nat};
func f1(x : A1<Bool>) : B1<Bool> = x;
func f2(x : A2<Bool>) : B2<Bool> = x;
func g1(x : A1<Bool>) : C1<Bool> = x;
func g2(x : A2<Bool>) : C2<Bool> = x;
};

()

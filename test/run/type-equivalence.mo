//MOC-FLAG -A=M0194
// Object Types

do {
type A = {x : Int};
type B = {x : Int};
func f(x : A) : A = x : B;
};

do {
type A = {x : Int; y : Bool};
type B = {y : Bool; x : Int};
func f(x : A) : A = x : B;
};

do {
type A = {x : Int; y : Bool};
type B = {y : Bool; x : Int};
type C = {x : {x : Int; y : Bool}; y : Bool};
type D = {y : Bool; x : B};
func f(x : C) : C = x : D;
};

// Function Types

do {
type A = (Int, Bool) -> (Nat8, Float);
type B = (Int, Bool) -> (Nat8, Float);
func f(x : A) : A = x : B;
};

do {
type A = <X> X -> X;
type B = <Y> Y -> Y;
func f(x : A) : A = x : B;
};

do {
type A = <X, Y> X -> Y;
type B = <Y, X> Y -> X;
func f(x : A) : A = x : B;
};


// Type Abbreviations

do {
type T<X, Y> = X;
type A = T<Int, Bool>;
type B = T<Int, Nat>;
func f(x : A) : A = x : B;
};


// Classes

do {
class C<X>() {};
type A = C<Int>;
type B = C<Int>;
func f(x : A) : A = x : B;
};

do {
class C<X, Y>() {};
type A<X> = C<X, Int>;
type B<X> = C<X, Int>;
func f<X>(x : A<X>) : A<X> = x : B<X>;
};

do {
class C<X>() {};
type T<X, Y> = X;
type A<X> = C<T<X, Int>>;
type B<X> = C<T<X, Bool>>;
func f<X>(x : A<X>) : A<X> = x : B<X>;
};


// Bounds

do {
type A = <X <: Int, Y <: {}> X -> Y;
type B = <X <: Int, Y <: {}> X -> Y;
func f(x : A) : A = x : B;
};

do {
type T<X, Y> = X;
type A = <X <: Int, Y <: Int> X -> Y;
type B = <X <: Int, Y <: T<Int, Bool>> X -> Y;
func f(x : A) : A = x : B;
};

do {
type T<X, Y> = X;
type A = <X <: Int, Y <: X> X -> Y;
type B = <X <: Int, Y <: T<X, Int>> X -> Y;
func f(x : A) : A = x : B;
};

do {
class C<X>() {};
type A = <X <: C<X>> X -> X;
type B = <X <: C<X>> X -> X;
func f(x : A) : A = x : B;
};

do {
class C<X>() {};
type A = <X <: C<Y>, Y <: C<X>> X -> Y;
type B = <X <: C<Y>, Y <: C<X>> X -> Y;
func f(x : A) : A = x : B;
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
type A = {x : A};
type B = {x : B};
func f(x : A) : A = x : B;
};

do {
type A = {x : B};
type B = {x : A};
func f(x : A) : A = x : B;
};

do {
type A0 = {x : A0};
type A = {x : A0};
type B = {x : B};
func f(x : A) : A = x : B;
};

do {
type A1 = {x : A2};
type A2 = {x : A1};
type B = {x : B};
func f(x : A1) : A1 = x : B;
func g(x : A2) : A2 = x : B;
func h(x : A2) : A2 = x : A1;
};

do {
type A1 = {x : A2; y : Int};
type A2 = {x : A1; y : Int};
type B1 = {x : B2; y : Int};
type B2 = {x : B1; y : Int};
func f1(x : A1) : A1 = x : B1;
func f2(x : A1) : A1 = x : B2;
func g1(x : A2) : A2 = x : B1;
func g2(x : A2) : A2 = x : B2;
func h1(x : A2) : A2 = x : A1;
func h2(x : B2) : B2 = x : B1;
};

do {
type A1 = {x : A2; y : Int};
type A2 = {x : A1; z : Nat};
type B1 = {x : C2; y : Int};
type B2 = {x : C1; z : Nat};
type C1 = {x : B2; y : Int};
type C2 = {x : B1; z : Nat};
func f1(x : A1) : A1 = x : B1;
func f2(x : A2) : A2 = x : B2;
func g1(x : A1) : A1 = x : C1;
func g2(x : A2) : A2 = x : C2;
};

do {
type A = {x : {x : A}};
type B = {x : B};
func f(x : A) : A = x : B;
};

do {
type A = {x : {x : {x : A}}};
type B = {x : {x : B}};
func f(x : A) : A = x : B;
};

do {
type A1 = {x : A2};
type A2 = {x : A3};
type A3 = {x : A1};
type B1 = {x : B2};
type B2 = {x : B1};
func f(x : A1) : A3 = x : B2;
func g(x : A2) : A1 = x : B1;
func h(x : A3) : A2 = x : B2;
};

do {
type A1<T> = {x : A2<T>; y : T};
type A2<T> = {x : A1<T>; z : Nat};
type B1<T> = {x : C2<T>; y : T};
type B2<T> = {x : C1<T>; z : Nat};
type C1<T> = {x : B2<T>; y : T};
type C2<T> = {x : B1<T>; z : Nat};
func f1(x : A1<Bool>) : A1<Bool> = x : B1<Bool>;
func f2(x : A2<Bool>) : A2<Bool> = x : B2<Bool>;
func g1(x : A1<Bool>) : A1<Bool> = x : C1<Bool>;
func g2(x : A2<Bool>) : A2<Bool> = x : C2<Bool>;
};

do {
type A1<T, U> = {x : A2<T, U>; y : T};
type A2<T, U> = {x : A1<T, U>; z : U};
type B1<T, U> = {x : C2<U, T>; y : T};
type B2<T, U> = {x : C1<U, T>; z : T};
type C1<T, U> = {x : B2<U, T>; y : T};
type C2<T, U> = {x : B1<U, T>; z : T};
func f1(x : A1<Nat, Bool>) : A1<Nat, Bool> = x : B1<Nat, Bool>;
func f2(x : A2<Nat, Bool>) : A2<Nat, Bool> = x : B2<Bool, Nat>;
func g1(x : A1<Nat, Bool>) : A1<Nat, Bool> = x : C1<Nat, Bool>;
func g2(x : A2<Nat, Bool>) : A2<Nat, Bool> = x : C2<Bool, Nat>;
};

()

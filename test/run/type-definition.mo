//MOC-FLAG -A=M0194
type IntT = Int;
type Arrow<T,U> = T->U;
type Compose<T,U,V> = (T->U) -> (U->V) -> T -> V;
type Id = <T> T->T;
type Seq<A> = (A, ?Seq<A>);
type A = (B,A);
type B = (A,B);
type None = ();  // shadowing prelude

// Test printing of shadowed type constructors
class C() {};
type T<X> = {f : <C> X -> C};
class D(x : T<C>) {let r = x.f};

type U<X> = {f : <Y> X -> Y};
class E<Y>(x : U<Y>) {let r = x.f};

// This is an expected forward reference error
//class F(x : FF) {r = x.a};
//type FF = {a : Int};

type AA = {b : BB};
type BB = {a : ?AA};

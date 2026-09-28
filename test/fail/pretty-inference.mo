// test pretty printing of inference errors
func p<T, U>(x : T, y: U) : (T, U) { (x, y); };
let p1 = p(1, true);

let p2 = p(p1, p1);
let p3 = p(p2, p2);
let p4 = p(p3, p3);
let p5 = p(p4, p4);

func f<T <: ()>(x:T) {};

f(1);

f(p1);

f(p2);

f(p3);

f(p4);

f(p5);

type C<X> = (X, X);
func g<T <: C<C<C<C<Nat>>>>>(x:T) {};

g(1);

g(p1);

g(p2);

g(p3);

g(p4);

g(p5);

func h<T, U>(x : T, _ : U -> ()) {};

h(1, "");

h(p1, "");

h(p2, "");

h(p3, "");

h(p4, "");

h(p5, "");

();

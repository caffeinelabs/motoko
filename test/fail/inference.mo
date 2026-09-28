func id<T>(x : T) : T { x };
ignore id(1);
ignore id<Nat>(1);

func swap<T, U>(x : T, y : U) : (U, T){ (y, x) };
ignore swap(1, true);
ignore swap(true, 1);

type N = <T>(T->T, T)->T;

func zero<T>(f : T->T, z : T) : T { z };

func succ(n : N) : N {
  func <T>(f : T->T, z:T) : T { f(n/*<T>*/(f, z)) }
};

ignore zero;
ignore succ(zero);
ignore succ(succ zero);

func app<T, U>(f : T-> U, x : T) : U { f(x); };

ignore app/*<N, N>*/(succ, zero);

func compose<T, U, V>(f: T -> U, g : U-> V) : T->V { func x { g (f(x)) } };

ignore compose((func(x : Int) : Int = x), (func (x : Int) : Int = x));
ignore compose/*<N,N,N>*/(succ, succ);

type List<T> = ?(T, List<T>);
type List2<T> = ?(T, List2<T>);

func cons<T>(h:T, t:List<T>):List<T> { ?(h, t) };
func nil<T>():List<T>{ null };

ignore nil<Nat>();
ignore cons(1, nil<Nat>());
ignore cons(1, cons(2, nil<Nat>()));
ignore cons(1, cons(2, null : List<Nat>));
ignore cons(1, cons(2, null : List2<Nat>:List<Nat>));
ignore cons(1, cons(2, null : List2<Nat>));

func curry<A, B, C>(f : (A, B)->C) : A->B->C {
  func(a:A): B->C { func(b:B):C { f(a, b) } }
};

func add(m:N, n:N) : N { m/*<N>*/(succ, n) };

func mult1(m:N, n:N): N { m/*<N>*/(func (a:N) : N { add(a, n) }, zero) };

func mult2(m:N, n:N) : N { m/*<N>*/(curry/*<N,N,N>*/ add n, zero) };

func tricky<T>(f : T -> T) {};

//explicit instantiation
tricky<None>(func f(x : Any) : None { f(x); });
tricky<Any>(func f(x : Any) : None { f(x); });
tricky<None>(func f(x : None) : Any { f(x); }); // correctly rejected
tricky<Any>(func f(x : None) : Any { f(x); }); // correctly rejected
ignore func <T>()  { tricky<T>(func f(x:Any):None{ f(x) }) };

// implicit instantiation
tricky(func f(x : Any) : None { f(x); }); // fails, under-constrained
tricky(func f(x : None) : Any { f(x); }); // fails, inconsistent instantiation required.
tricky(func f(x : None) : None { f(x); });

func amb<T>(f : T -> T): T->T { f };
ignore amb<None>(func f(x : Any) : None { f(x); }) : None -> None;
ignore amb<Any>(func f(x : Any) : None { f(x); }) : Any -> Any;

func co<T>(x : T, y : T) : () {};
co<Nat>(1, 2);
co<Int>(1, 2 : Int);
co<Any>(1, true);
co(1, 2);
co(1, 2:Int);
co(1, true);

func contra<T>(f : (T, T) -> ()) : () {};
contra<Nat>(func (x : Nat, y : Nat) {});
contra<Nat>(func (x : Nat, y : Int) {});
contra<None>(func (x : Nat, y : Bool) {});
contra(func (x : Nat, y : Nat) {});
contra(func (x : Nat, y : Int) {});
contra(func (x : Nat, y : Bool) {});

func coswap<T <: U, U>(x : T, y : T): (U, U) { (y, x) };
ignore coswap(1, 2); // rejected due to open bounds
ignore coswap(1, 2 : Int); // rejected due to open bounds
ignore coswap(1, true); // rejected due to open bounds

// support domain driven overloading for implicitly scoped,
// otherwise monomorphic functions (as before)
func f(g:shared Nat8 -> ()) : async () {
  g(1);
};

func bnd<T <: Int>(x : T) : T { x };
ignore bnd(1 : Int) : Int;
ignore bnd(1) : Nat; // ok, uses expected type
ignore (if false { (bnd(loop {}):Nat) } else { 1 }); // ok, given expected type
ignore (if false { (bnd(loop {}):Int) } else { 1 });
ignore (if false { (bnd(loop {}):None) } else { 1 }); // ok, given expected type
ignore bnd(true); // reject, overconstrained
bnd(true); // reject, overconstrained

// reject scope violations
func scopeco<T>(f:<U>T->U){};
scopeco(func<V>(x:V):V{ x }); // reject due to scope violation
func scopecontra<T>(f:<U>U->T){};
scopecontra(func<V>(x:V):V{ x }); // reject due to scope violation

//TODO: invariant mutables, constructor constraints, bail on open bounds

// reject instantiations at  `var _` (not denotable)
func sub<T>(x : [T]) : T { x[0] };
ignore sub([1]);
sub([]);
ignore sub([var 1]); // reject

func sub_mut<T>(x : [var T]) :T { x[0] };
sub_mut([1]); // reject
ignore sub_mut([var 1]); // ok
sub_mut([var 1]); // reject

// tricky examples involving open typing with bounded parameters
func g<T <: Int>(x : T) {
  func f<U <: {}>(y : U) {};
  f(x); // reject, overconstrained U
};

func h<T <: {}>(x : T) {
  func f<U <: {}>(y : U) {};
  f(x); // accept
};

func i<T <: Any>(x : T) {
  func f<U <: T>(y : U) : U { y };
  ignore f(x); // accept
};

func j<T <: Any>(x : T) {
  func f<U <: T>(y : U) : U { y };
  ignore f(x) : None; // fail (overconstrained)
};

func k<T <: Any>(x:T) {
  func f<U <: T>(y : U) : U { y };
  ignore f(x) : T;
};

// immutable arrays

func choose<T>(b : Bool, x : [T], y : [T]) : [T] { if b { x } else { y } };
ignore choose(true, [1 : Nat], [1 : Nat]);
ignore choose(true, [1 : Int], [1 : Int]);
ignore choose(true, [1 : Nat], [1 :  Int]);

// mutable arrays
func choose_var<T>(b : Bool, x : [var T], y : [var T]) : [var T] { if b { x } else { y } };
ignore choose_var(true, [var (1:Nat)], [var (1:Nat)]);
ignore choose_var(true, [var (1:Int)], [var (1:Int)]);
ignore choose_var(true, [var (1:Nat)], [var (1:Int)]); // rejected as overconstrained (variance not applicable)

// reject type components as (non denotable) instantiations
func select<T>(o : object { x : T }) : T { o.x };
ignore select(object { public let x = 1 });
ignore select(object { public type x = Nat }); // reject

// distinguish empty <> from omitted instantiation, no inference when empty
func fst<T, U>((t, u) : (T, U)) : T{ t };
ignore fst((1, 2));
ignore fst<Nat, Int>((1, 2));
ignore fst<>((1, 2)); // reject

// more return type driven inference (no need to instantiate nil())
ignore nil<>() : List<Nat>; // reject
ignore nil() : List<Nat>;
ignore cons(1, nil()) : List<Nat>;
ignore cons(1, cons(2, nil())) : List<Nat>;

func req(x : Nat) : async Int { return x };
func send<T <: Nat>(f: Nat -> async Int, x : T) : async Int { await f(x); };
async { ignore send(req, 0); };

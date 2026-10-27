#!/usr/bin/env bash
# Tests pretty printing of values and types
moc -W M0215 -i <<__END__
import Prim "mo:⛔";

let a_small = Prim.Array_init<Text>(5,"hello");
let a_large = Prim.Array_init<Text>(100,"hello");

let a_small = Prim.Array_tabulate<Nat>(5,func i { i });
let a_large = Prim.Array_tabulate<Nat>(100,func i { i } );

func a<T>(x : T, y : T) : [T] { [x,y] } ;
let a1 = a(#A, #B);
let a2 = a(a1, a1);
let a3 = a(a2, a2);
let a4 = a(a3, a3);
let a5 = a(a4, a4);

func r<T,U>(x:T, y:U) : {A : T; B : U } { {A = x; B = y} } ;
let r1 = r(#A, #B);
let r2 = r(r1, r1);
let r3 = r(r2, r2);
let r4 = r(r3, r3);
let r5 = r(r4, r4);

func m<T,U>(x:T, y:U) : module {A : T; B : U } {
  module { public let (A,B) = (x,y)} } ;
let m1 = m(#A, #B);
let m2 = m(m1, m1);
let m3 = m(m2, m2);
let m4 = m(m3, m3);
let m5 = m(m4, m4);

func f<T,U>(f : T->U) : (U->T)->(T->U) { func g {f}; } ;
let f1 = f(func (#A) : {#B} {#B});
let f2 = f(f1);
let f3 = f(f2);
let f4 = f(f3);
let f5 = f(f4);

func p<T,U>(x : T, y : U) : (T,U) { (x,y) } ;
let p1 = p(#A, #B);
let p2 = p(p1, p1);
let p3 = p(p2, p2);
let p4 = p(p3, p3);
let p5 = p(p4, p4);

func v<T,U>(x:T, y:U): {#A : T; #B : U } { #A x } ;
let v1 = v(#Foo, #Bar);
let v2 = v(v1, v1);
let v3 = v(v2, v2);
let v4 = v(v3, v3);
let v5 = v(v4, v4);

func o<T,U>(x:T): ?T { ?x } ;
let o1 = o(666);
let o2 = o(o1);
let o3 = o(o2);
let o4 = o(o3);
let o5 = o(o4);

class c<T,U>(x:T, y:U) {
  public type List<T> = ?(T, List<T>);
  public let (A,B) = (x,y)
} ;
let c1 = c(#A, #B);
let c2 = c(c1, c1);
let c3 = c(c2, c2);
let c4 = c(c3, c3);
let c5 = c(c4, c4);

// subtyping hides values
let a : Any = 1;

let r : {a : Nat} = { a = 1; hidden = true};

let r : Any = { a = 1; hidden = true};

__END__

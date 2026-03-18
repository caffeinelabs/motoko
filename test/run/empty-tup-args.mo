//MOC-FLAG -A=M0194
func f() {};
func g(()) {};
func g1((())) {}; // only top-level parenthesis signifies arity
func h(u:()) {};
func h1((u:())) {}; // dito, intermediate parentheses are redundant

let _ = f : () -> ();
let _ = g : (()) -> ();
let _ = h : (()) -> ();

let _ : [() -> ()] = [f, func () {}, func f1 () {}];
let gh : [(()) -> ()] = [g, g1, h, h1, func (()) {}, func g2 (()) {}];

f();
g(); // accepted, because "At calls there is an implicit coercion
     //                    between tuples and n-ary arguments"
h(); // dito
gh[0](); // dito
gh[1](); // dito

// Correct:
g(());
h(());
h1(());
gh[0](());
gh[1](());
gh[2](());
gh[3](());

func j(a:Int) {};
func k(a:(Int)) {};
func l(a:((Int))) {};

let jkl : [Int -> ()] = [j, k, l, func l1(a:((Int))) {}];

j(5);
k(8);
l(13);
jkl[1](21);

func m((a:Int, b:Bool)) {};
func m1((a:Int, b:Bool,)) {};
let ms : [((Int, Bool)) -> ()] = [m, m1];

func n(a:Int, b:Bool) {};
func n1(a:Int, b:Bool,) {};

let ns : [(Int, Bool) -> ()] = [n, n1];

func o((a:Int,)) {};
let os : [((Int,)) -> ()] = [o];

// class tests, these have the same argument regime
class F() {};
class G(()) {};
class H(a:()) {};
class G1((())) {};
class H1((u:())) {};
let _ : [() -> F] = [F];
let _ : [(()) -> G] = [G];
let _ : [(()) -> G1] = [G1];
let _ : [(()) -> H] = [H];
let _ : [(()) -> H1] = [H1];

func annih<A,B>(f : A -> B) : (A -> ()) { func (a : A) { ignore (f a) } };
let _ : [(()) -> ()] = [annih<(),G> G, annih<(),G1> G1, annih<(),H> H, annih<(),H1> H1];


// test that parens are not significant deeper into the pattern,
// even around tuples

func p(a:Int,  () ) {};
func q(a:Int, (())) {};
func r(a:Int,  (b:Int,) ) {};
func s(a:Int, ((b:Int,))) {};
let _ : [(Int, ()) -> ()] = [p, q];
let _ : [(Int, (Int,)) -> ()] = [r, s];

// redundant parens for anonymous functions

let _ = (func (a:Nat, ()) : Nat { a }) (42, ());
let _ = (func ((a:Nat, (()))) : Nat { a }) (42, ());

// test that switch expressions also behave correctly with redundant parens

let _ = switch (42,) { case (3,) 3; case ((5,)) 5; case (a,) a};
let _ = switch (42,) { case (3,) 3; case ((5,)) 5; case (((a,))) a};

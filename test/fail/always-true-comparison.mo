//MOC-FLAG -A=M0194
// `==`/`!=` whose only common type has a single value
func f1(n : Nat, t : Text) : Bool = n == t;
func f2(n : Nat, t : Text) : Bool = n != t;
func f3(a : { id : Nat }, b : { userId : Nat }) : Bool = a == b;
func f4(a : { x : { p : Nat } }, b : { x : { q : Nat } }) : Bool = a == b;
func f5(a : ({ p : Nat }, {}), b : ({ q : Nat }, { r : Text })) : Bool = a != b;
func f6(a : {#a}, b : {#a : Nat}) : Bool = a == b;
func f7<A, B>(a : A, b : B) : Bool = a == b;
func f8<A>(a : A, b : A) : Bool = a == b;

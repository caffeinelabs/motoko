//MOC-FLAG -A=M0194
// comparisons at a type with a single value
func f1(n : Nat, t : Text) : Bool = n == t;
func f2(n : Nat, t : Text) : Bool = n != t;
func f3(a : { id : Nat }, b : { userId : Nat }) : Bool = a == b;
func f4(a : { x : { p : Nat } }, b : { x : { q : Nat } }) : Bool = a == b;
func f5(a : ({ p : Nat }, {}), b : ({ q : Nat }, { r : Text })) : Bool = a != b;
func f6(a : {#a}, b : {#a : Nat}) : Bool = a == b;
func f7<A, B>(a : A, b : B) : Bool = a == b;
func f8<A>(a : A, b : A) : Bool = a == b;
func f9(a : {}, b : {}) : Bool = a == b;
func f10(a : Any, b : Any) : Bool = a != b;
func f11(a : Null, b : Null) : Bool = a == b;
func f12(a : (), b : ()) : Bool = a == b;

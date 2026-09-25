// simplest example of failure (negative version)
let rec1 : { var x : Nat } = { var x = (0 : Int) }; // reject
let rec1a : { var x : Int } = { x = (0 : Nat) }; // reject
let rec1b : { x : Int } = { var x = (0 : Nat) }; // reject

// repro from #3265 (negative version)
type List<X> = ?(X, List<X>);
func nil<X>() : List<X> = null;
func sub(x : List<None>) : List<Nat>{ x };
let rec2 : { var myList : List<None> } = { var myList = nil<Nat>() }; // reject
let rec2a : { myList : List<Nat> } = { var myList = nil<None>() }; // reject
let rec2b : { var myList : List<Nat> } = { myList = nil<None>() }; // reject

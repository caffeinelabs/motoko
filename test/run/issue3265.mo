//MOC-FLAG -A=M0194
// simplest example of failure
let rec1 : { var x : Int } = { var x = (0 : Nat)};

// repro from #3265
type List<X> = ?(X, List<X>);
func nil<X>() : List<X> = null;
func sub(x : List<None>) : List<Nat>{x};
let rec2 : { var myList : List<Nat> } = { var myList = nil<None>() };

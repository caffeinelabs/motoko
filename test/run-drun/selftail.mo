import Prim "mo:⛔";
// a single function that can be evaluated recursively or tail-recursively
func f (tailCall:Bool, n:Int, acc:Int) : Int {
  if n <= 0
  { return acc };

  if tailCall
  { f(tailCall, n - 1, acc + 1) }
  else
  { 1 + f(tailCall, n - 1, acc) };
};

// check we get same results for small n
assert (f(false, 100, 0) == f(true, 100, 0));
Prim.debugPrint "ok1";

// check tail recursion works for large n
assert(200000 == f (true, 200000, 0));
Prim.debugPrint "ok2";

// check recursion overflows for large n (overflows on drun only)
assert(200000 == f (false, 200000, 0));
Prim.debugPrint "ok3 (unreachable on drun)";

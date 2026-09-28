import Prim "mo:⛔";

// fails in the interpreter at line * with  ../../src/moc -r  u.mo
// fails in the interpreter at line * with  ../../src/moc -r -a u.mo
// but succeeds  ../../src/moc -r -a -A u.mo

// the problem is that the type-checker accepts line * because  (Int,) and Int domain types
// aren't distinguished after arity raising, since Type.mo_seq Int = Type.mo_seq (Tup[Int]),
// eventhough Int != Tup[Int]

do {
  let t = "u_u";
  shared func fu_u(a:Int,) : async (Int,) {
    return (2 * a,);
  };

  let _ : async (Int,)  = async {
    let (x,) = await fu_u(1); // *
    assert(x == 2);
    Prim.debugPrint t;
    return (x,);
  };
};

import P "mo:prim";

do {
  let b : Blob = to_candid(
    (#a 1) :
    ({#a:Int})
  );
  let o = (from_candid b) :?({#a:Int});
  P.debugPrint(debug_show(o));
  assert o == ? (#a 1);
}; // ok

do {
  let b : Blob = to_candid(
    (#a 1) :
    ({#a:Int})
  );
  let o = (from_candid b) :?({#b:Int});
  P.debugPrint(debug_show(o));
  assert o == null;
}; // ok

do {
  let b : Blob = to_candid(
    (#b 1) :
    ({#b:Int})
  );
  let o = (from_candid b) :?({#b:Nat});
  P.debugPrint(debug_show(o));
  assert o == null;
};


do {
  let b : Blob = to_candid(
    [#b 1] :
    [{#b:Int}]
  );
  let o = (from_candid b) :?[{#b:Nat}];
  P.debugPrint(debug_show(o));
  assert o == null;
}; // ok


do {
  let b : Blob = to_candid(
    ?[1] :
    ?[Int]
  );
  let o = (from_candid b) :??[Nat];
  P.debugPrint(debug_show(o));
  assert o == ?null;
}; // ok

do {
  let b : Blob = to_candid(
    ?[1] :
    ?[Int]
  );
  let o = (from_candid b) :?[Nat];
  P.debugPrint(debug_show(o));
  assert o == null;
}; // ok


do {
  let b : Blob = to_candid(
    ?[#b 1] :
    ?[{#b:Int}]
  );
  let o = (from_candid b) :?[{#b:Nat}];
  P.debugPrint(debug_show(o));
  assert o == null;
}; // ok

//SKIP run
//SKIP run-ir
//SKIP run-low

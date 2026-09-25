let b : Blob = to_candid (
  ?[
    { a=?#a },
    { a=?#b{} },
    { a=?#b{} },
    { a=?#a },
    { a=?#c },
    { a=?#a }
  ] : ?[{ a:?{ #a; #b:{}; #c } }]);

let o1 = (from_candid b) : ?[{ a:?{ #a } }]; //note missing ?, forcing decoding at incorrect array type
assert o1 == null;

let o2 = (from_candid b) : ??[{ a:?{ #a } }]; //intended example, with embedded defaulting
assert o2 ==
  ??[
    { a=?#a },
    { a=null },
    { a=null },
    { a=?#a },
    { a=null },
    { a=?#a }
  ];

//SKIP run
//SKIP run-ir
//SKIP run-low

// assert blob "DIDL\00\02\7d\7d\05\06" == "(5, 6)" : (nat, nat, null) "parsing a top-level tuple into a longer top-level tuple";
do {
  let candid = to_candid(5, 6);
  assert "DIDL\00\02\7d\7d\05\06" == candid;
  let ?(5, 6, null) : ?(Nat, Nat, Null) = from_candid(candid) else loop { () };
};

// assert blob "DIDL\01\6c\00\01\00" == "(record { })"                               : (record {2:null}) "record: missing null field also in textual format";
do {
  let candid = to_candid({});
  assert "DIDL\01\6C\00\01\00" == candid;
  let ?{ _2_ = null } : ?{ _2_ : Null } = from_candid(candid) else loop { () };
};

// assert blob "DIDL\01\6c\01\00\7d\01\00\05" == "(record { 0 = 5 })" : (record { 1 : null }) "parsing into record with expected field that is greater than extra field on the wire";
do {
  let candid = to_candid({ _0_ = 5 });
  assert "DIDL\01\6c\01\00\7d\01\00\05" == candid;
  let ?{ _1_ = null } : ?{ _1_ : Null } = from_candid(candid) else loop { () };
};

// assert blob "DIDL\01\6c\01\01\7d\01\00\05" == "(record { 1 = 5 })" : (record { 0 : null }) "parsing into record with expected field that is less than extra field on the wire";
do {
  let candid = to_candid({ _1_ = 5 });
  assert "DIDL\01\6c\01\01\7d\01\00\05" == candid;
  let ?{ _0_ = null } : ?{ _0_ : Null } = from_candid(candid) else loop { () };
};

//SKIP run
//SKIP run-ir
//SKIP run-low

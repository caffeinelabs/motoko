import Prim "mo:prim";

actor {

let ?o1 = from_candid(to_candid({x = (5 : Nat32)})) :?({x : ?Nat32});
let ?o2 = from_candid(to_candid({x = (5 : Nat32)})) :?({x : ??Nat32});
let ?o3 = from_candid(to_candid({x = (5 : Nat32)})) :?({x : ???Nat32});

Prim.debugPrint (debug_show {o1;o2;o3});

assert o1.x == ?5; // succeeds
assert o2.x == ??5; // fails (but should now succeed)
assert o3.x == ???5; // fails (but should now succeed)

// Remaining tests adapted from Rust PR

// Deserialize `null : null` to `opt null`, `opt opt null`, and `opt opt opt null`.
do {
  let ?o1 = from_candid(to_candid({x = (null : Null)})) :?({x : ?Null});
  let ?o2 = from_candid(to_candid({x = (null : Null)})) :?({x : ??Null});
  let ?o3 = from_candid(to_candid({x = (null : Null)})) :?({x : ???Null});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == (null : ?Null);
  assert o2.x == (null : ??Null);
  assert o3.x == (null : ???Null);
};


// Deserialize `5 : nat64` to `opt null`, `opt opt null`, and `opt opt opt null`.
do {

  let ?o1 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ?Null});
  let ?o2 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ??Null});
  let ?o3 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ???Null});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == ?null;
  assert o3.x == ??null;
};


// Deserialize `null : opt nat64` to `opt null`, `opt opt null`, and `opt opt opt null`.
do {

  let ?o1 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ?Null});
  let ?o2 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ??Null});
  let ?o3 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ???Null});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == null;
  assert o3.x == null;
};

// Deserialize `opt 5 : opt nat64` to `opt null`, `opt opt null`, and `opt opt opt null`.
do {

  let ?o1 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ?Nat64});
  let ?o2 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ??Nat64});
  let ?o3 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ???Nat64});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == ?5;
  assert o2.x == ??5;
  assert o3.x == ???5;
};

// Deserialize `null : reserved` to `opt reserved`, `opt opt reserved`, and `opt opt opt reserved`.
do {

  let ?o1 = from_candid(to_candid({x = (null : Any)})) :?({x : ?Any});
  let ?o2 = from_candid(to_candid({x = (null : Any)})) :?({x : ??Any});
  let ?o3 = from_candid(to_candid({x = (null : Any)})) :?({x : ???Any});

  Prim.debugPrint ("// can't `debug_show {o1;o2;o3}` involving Any");

  assert o1.x == null;
  assert o2.x == null;
  assert o3.x == null;
};

// Deserialize `5 : nat64` to `opt reserved`, `opt opt reserved`, and `opt opt opt reserved`.
do {

  let ?o1 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ?Any});
  let ?o2 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ??Any});
  let ?o3 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ???Any});

  Prim.debugPrint ("// can't `debug_show {o1;o2;o3}` involving Any");

  let any = ();
  assert o1.x == ?any;
  assert o2.x == ??any;
  assert o3.x == ???any;

};


// Deserialize `null : opt nat64` to `opt reserved`, `opt opt reserved`, and `opt opt opt reserved`.
do {

  let ?o1 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ?Any});
  let ?o2 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ??Any});
  let ?o3 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ???Any});

  Prim.debugPrint ("// can't `debug_show {o1;o2;o3}` involving Any");

  assert o1.x == null;
  assert o2.x == null;
  assert o3.x == null;
};


// Deserialize `opt 5 : opt nat64` to `opt reserved`, `opt opt reserved`, and `opt opt opt reserved`.
do {

  let ?o1 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ?Any});
  let ?o2 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ??Any});
  let ?o3 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ???Any});

  Prim.debugPrint ("// can't `debug_show {o1;o2;o3}` involving Any");

  let any = ();
  assert o1.x == ?any;
  assert o2.x == ??any;
  assert o3.x == ???any;

};

// Deserialize `5 : nat64` to `opt nat64`, `opt opt nat64`, and `opt opt opt nat64`.
do {

  let ?o1 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ?Nat64});
  let ?o2 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ??Nat64});
  let ?o3 = from_candid(to_candid({x = (5 : Nat64)})) :?({x : ???Nat64});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == ?5;
  assert o2.x == ??5;
  assert o3.x == ???5;
};


// Deserialize `null : opt nat64` to `opt nat64`, `opt opt nat64`, and `opt opt opt nat64`.
do {

  let ?o1 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ?Nat64});
  let ?o2 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ??Nat64});
  let ?o3 = from_candid(to_candid({x = (null : ?Nat64)})) :?({x : ???Nat64});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == null;
  assert o3.x == null;
};


// Deserialize `opt 5 : opt nat64` to `opt nat64`, `opt opt nat64`, and `opt opt opt nat64`.
do {

  let ?o1 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ?Nat64});
  let ?o2 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ??Nat64});
  let ?o3 = from_candid(to_candid({x = (?5 : ?Nat64)})) :?({x : ???Nat64});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == ?5;
  assert o2.x == ??5;
  assert o3.x == ???5;
};

// Deserialize `5 : nat` to `opt int`, `opt opt int`, and `opt opt opt int`.
do {

  let ?o1 = from_candid(to_candid({x = (?5 : ?Nat)})) :?({x : ?Int});
  let ?o2 = from_candid(to_candid({x = (?5 : ?Nat)})) :?({x : ??Int});
  let ?o3 = from_candid(to_candid({x = (?5 : ?Nat)})) :?({x : ???Int});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == ?5;
  assert o2.x == ??5;
  assert o3.x == ???5;
};


// Deserialize `null : opt nat` to `opt int`, `opt opt int`, and `opt opt opt int`.
do {

  let ?o1 = from_candid(to_candid({x = (null : ?Nat)})) :?({x : ?Int});
  let ?o2 = from_candid(to_candid({x = (null : ?Nat)})) :?({x : ??Int});
  let ?o3 = from_candid(to_candid({x = (null : ?Nat)})) :?({x : ???Int});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == null;
  assert o3.x == null;
};

// Deserialize `opt 5 : opt nat` to `opt int`, `opt opt int`, and `opt opt opt int`.
do {

  let ?o1 = from_candid(to_candid({x = (?5 : ?Nat)})) :?({x : ?Int});
  let ?o2 = from_candid(to_candid({x = (?5 : ?Nat)})) :?({x : ??Int});
  let ?o3 = from_candid(to_candid({x = (?5 : ?Nat)})) :?({x : ???Int});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == ?5;
  assert o2.x == ??5;
  assert o3.x == ???5;
};


// Deserialize `5 : int` to `opt nat`, `opt opt nat`, and `opt opt opt nat`.
do {

  let ?o1 = from_candid(to_candid({x = (5 : Int)})) :?({x : ?Nat});
  let ?o2 = from_candid(to_candid({x = (5 : Int)})) :?({x : ??Nat});
  let ?o3 = from_candid(to_candid({x = (5 : Int)})) :?({x : ???Nat});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == ?null;
  assert o3.x == ??null;
};


// Deserialize `null : ?int` to `opt nat`, `opt opt nat`, and `opt opt opt nat`.
do {

  let ?o1 = from_candid(to_candid({x = (null : ?Int)})) :?({x : ?Nat});
  let ?o2 = from_candid(to_candid({x = (null : ?Int)})) :?({x : ??Nat});
  let ?o3 = from_candid(to_candid({x = (null : ?Int)})) :?({x : ???Nat});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == null;
  assert o3.x == null;
};


// Deserialize `opt 5 : opt int` to `opt nat`, `opt opt nat`, and `opt opt opt nat`.
do {

  let ?o1 = from_candid(to_candid({x = (?5 : ?Int)})) :?({x : ?Nat});
  let ?o2 = from_candid(to_candid({x = (?5 : ?Int)})) :?({x : ??Nat});
  let ?o3 = from_candid(to_candid({x = (?5 : ?Int)})) :?({x : ???Nat});

  Prim.debugPrint (debug_show {o1;o2;o3});

  assert o1.x == null;
  assert o2.x == ?null;
  assert o3.x == ??null;
  };


// Do not deserialise `reserved` to `Null`.
label good do {let ?((null, 5), null) : ?((Null, Nat), Null) = from_candid "DIDL\01\6C\02\00\7F\01\7D\02\00\7C\05\7A" else break good; assert false};
label good do {let ?(null, null) : ?(Null, Null) = from_candid "DIDL\00\02\70\7D\05" else break good; assert false};
label good do {let ?(null, 5) : ?(Null, Nat) = from_candid "DIDL\00\02\70\7D\05" else break good; assert false};
label good do {let ?null : ?Null = from_candid "DIDL\00\01\70" else break good; assert false};


public func go(null) : () {
  assert false; // should never arrive here
};

public func go2(null, null) : () {
  assert false; // should never arrive here
};

}

//CALL ingress go "DIDL\x00\x01\x70"
//CALL ingress go "DIDL\x00\x01\x7D\x05"
//CALL ingress go "DIDL\x00\x02\x70\x7D\x05"

//CALL ingress go2 "DIDL\x00\x02\x70\x70"
//CALL ingress go2 "DIDL\x00\x02\x70\x7F"
//CALL ingress go2 "DIDL\x00\x02\x70\x7D\x05"

//SKIP run
//SKIP run-ir
//SKIP run-low

//MOC-FLAG -A M0239

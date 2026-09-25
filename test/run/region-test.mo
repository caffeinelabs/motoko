import Prim "mo:⛔";
import Region "stable-region/Region";

let r = Region.new();
let 0 = Region.size(r);
let 16 = Region.id(r);
let 0 = Region.grow(r, 64);
assert (64 == Region.size(r));

do {
  Prim.debugPrint("Nat8");
  type T = Nat8;
  let size : Nat64 = 1;
  let mod : Nat64 = 256;
  let load = Region.loadNat8;
  let store = Region.storeNat8;
  func conv(n : Nat64) : Nat8 { Prim.natToNat8(Prim.nat64ToNat(n % mod)) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Nat16");
  type T = Nat16;
  let size : Nat64 = 2;
  let mod : Nat64 = 65536;
  let load = Region.loadNat16;
  let store = Region.storeNat16;
  func conv(n : Nat64) : Nat16 { Prim.natToNat16(Prim.nat64ToNat(n % mod)) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Nat32");
  type T = Nat32;
  let size : Nat64 = 4;
  let mod : Nat64 = 2**32;
  let load = Region.loadNat32;
  let store = Region.storeNat32;
  func conv(n : Nat64) : Nat32 { Prim.natToNat32(Prim.nat64ToNat(n % mod)) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Nat64");
  type T = Nat8;
  let size : Nat64 = 8;
  let load = Region.loadNat64;
  let store = Region.storeNat64;
  func conv(n : Nat64) : Nat64 { n };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Int8");
  type T = Int8;
  let size : Nat64 = 1;
  let mod : Nat64 = 256;
  let load = Region.loadInt8;
  let store = Region.storeInt8;
  func conv(n : Nat64) : Int8 { Prim.intToInt8(Prim.nat64ToNat(n % mod) - 128) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Int16");
  type T = Int16;
  let size : Nat64 = 2;
  let mod : Nat64 = 65536;
  let load = Region.loadInt16;
  let store = Region.storeInt16;
  func conv(n : Nat64) : Int16 { Prim.intToInt16(Prim.nat64ToNat(n % mod) - 32768) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Int32");
  type T = Int32;
  let size : Nat64 = 4;
  let mod : Nat64 = 2**32;
  let load = Region.loadInt32;
  let store = Region.storeInt32;
  func conv(n : Nat64) : Int32 { Prim.intToInt32(Prim.nat64ToNat(n % mod) - 2147483648) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Int64");
  type T = Int8;
  let size : Nat64 = 8;
  let load = Region.loadInt64;
  let store = Region.storeInt64;
  func conv(n : Nat64) : Int64 { Prim.nat64ToInt64(n) + (-9223372036854775808) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Float");
  type T = Int8;
  let size : Nat64 = 8;
  let load = Region.loadFloat;
  let store = Region.storeFloat;
  func conv(n : Nat64) : Float { Prim.int64ToFloat(Prim.nat64ToInt64(n) + (-9223372036854775808)) };
  let max = Region.size(r) * 65536;
  var i : Nat64 = 0;
  while i < max {
    store(r, i, conv(i));
    i += size;
  };
  i := 0;
  while i < max {
    assert (conv(i) == load(r, i));
    i += size;
  };
};

do {
  Prim.debugPrint("Blob");
  type T = Int8;
  let load = Region.loadBlob;
  let store = Region.storeBlob;
  func conv(n : Nat64) : Blob { load(r, 0, Prim.nat64ToNat(n)) };
  let max : Nat64 = Region.size(r) * 65536;
  var i : Nat64 = 1;
  while i < max {
    let b = conv(i);
    store(r, 0, b);
    assert (b == load(r, 0, Prim.nat64ToNat(i)));
    i := i * 2;
  };
};

//SKIP run-low
//SKIP run
//SKIP run-ir

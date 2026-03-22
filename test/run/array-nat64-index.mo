import Prim "mo:prim";

// Test Nat64 indexing into immutable arrays
let a = [10, 20, 30, 40, 50];
let i0 : Nat64 = 0;
let i2 : Nat64 = 2;
let i4 : Nat64 = 4;

assert(a[i0] == 10);
assert(a[i2] == 30);
assert(a[i4] == 50);

// Test Nat64 indexing into mutable arrays
let b = [var 1, 2, 3, 4, 5];
let j1 : Nat64 = 1;
let j3 : Nat64 = 3;

assert(b[j1] == 2);
assert(b[j3] == 4);

// Test Nat64 index for mutation
b[j1] := 42;
assert(b[j1] == 42);

b[j3] := 99;
assert(b[j3] == 99);

// Test mixing Nat and Nat64 indices on the same array
assert(b[0] == 1);
assert(b[i0] == 1);

// Test Nat64 index in a loop
let c = [var 0, 0, 0, 0, 0, 0, 0, 0];
var k : Nat64 = 0;
while (k < 8) {
  c[k] := Prim.nat64ToNat(k * 10);
  k += 1;
};

assert(c[0] == 0);
assert(c[(3 : Nat64)] == 30);
assert(c[(7 : Nat64)] == 70);

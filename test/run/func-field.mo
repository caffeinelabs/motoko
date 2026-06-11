// Test func-field syntax sugar: `func name(params) : ret = body` as a record field
// desugars at parse time to `name = func(params) : ret = body` (Phase 1 — no sibling capture)

let r = {
  base = 10;
  func add(x : Nat) : Nat = x + 1;
  func const() : Nat = 42;
};
assert (r.add(r.base) == 11);
assert (r.const() == 42);

// with-extension form
let b = { y = 5 };
let s = { b with func f() : Nat = 7 };
assert (s.y == 5 and s.f() == 7);

// block body form
let t = {
  func double(n : Nat) : Nat { n + n };
};
assert (t.double(6) == 12);

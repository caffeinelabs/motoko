//MOC-FLAG --moi-cache _out/moi-cache-dir
import C "moi-cache/counter";
import T "moi-cache/types";
import Tr "moi-cache/tree";
import { debugPrint } "mo:prim";

// Counter tests
let c = C.make();
C.inc(c);
C.inc(c);
C.inc(c);
assert C.get(c) == 3;

let p = C.pair<Nat, Text>(42, "hello");
assert p.0 == 42;
assert p.1 == "hello";

// Result/variant tests
let ok = T.wrapOk<Nat>(7);
let mapped = T.mapResult<Nat, Text>(ok, func(n : Nat) : Text { debug_show n });
switch mapped {
  case (#ok t) { assert t == "7"; debugPrint(t) };
  case (#err _) { assert false };
};

let err = T.wrapErr<Nat>("boom");
switch err {
  case (#ok _) { assert false };
  case (#err e) { assert e == "boom"; debugPrint(e) };
};

// Recursive type tests (tree)
let tree = Tr.node(
  Tr.node(Tr.leaf(), 1, Tr.leaf()),
  2,
  Tr.node(Tr.leaf(), 3, Tr.leaf()),
);
assert Tr.size(tree) == 3;

let mapped_tree = Tr.map<Nat, Text>(tree, func(n : Nat) : Text { debug_show n });
assert Tr.size(mapped_tree) == 3;

debugPrint("moi-cache test passed");

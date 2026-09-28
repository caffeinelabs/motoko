//MOC-FLAG --package pkg pkg
import _Unused1 "mo:pkg/unused1"; // don't report package imports usage warning
import _Unused2 "./pkg/unused2"; // do report relative imports usage warning

func rec1() { rec1() }; // accepted, but reject as unused?

func rec() {}; //reject ok

do { let unused = 1 };

func g(x : ()) {};

//let unused = 1;
let _ok = 1;
let hmm = _ok; // should we warn about the use of a silenced identifier? OCaml doesn't actually, so perhaps not

do {
  func f() { g(); }; //both f and g are only used recursively accept or reject?
  func g() { f() };
};

// switches
switch (?1) {
  case ?u {} //unused u
  case o {} //unused 0
};

// types

do { type Unused = Int; }; // do we want to warn?

let r = object { private let f = 0; public let x = 0 };

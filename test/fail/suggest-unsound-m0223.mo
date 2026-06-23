//MOC-FLAG -W=M0223,M0236,M0237 --all-libs --package core $MOTOKO_CORE
import List "mo:core/List";
import Array "mo:core/Array";
import Prim "mo:prim";

// 3 seemingly redundant instantiations but at least one is necessary!
func _reports_suggestions(n : Nat) : [List.List<Nat>] {
  Array.tabulate<List.List<Nat>>( // warning (M0223)
    n,
    func(i) = List.fromArray<Nat>( // NO warning (kept: the one necessary instantiation)
      Array.tabulate<Nat>(i + 1, func(j) = j) // warning (M0223)
    ),
  );
};

// dropping all three (the unsound over-application) no longer type-checks
func _applied_unsound_suggestions(n : Nat) : [List.List<Nat>] {
  Array.tabulate(
    n,
    func(i) = List.fromArray( // error (M0098): nothing pins the element type
      Array.tabulate(i + 1, func(j) = j)
    ),
  );
};

type R = { x : Int; y : Nat };

module Impl1 {
  public func impl(t : [var ?R]) : [var ?R] = t;
};

module Impl2 {
  public func impl(t : ?R) : ?R = t;
};

func withImpl<T>(n : Nat, impl : (implicit : T -> T), k : Nat -> T) : [var T] {
  ignore impl;
  [var k(n)];
};

// 2 x 2 matrix of M0223 and M0237 with seemingly two redundant instantiation/implict arg
// but only one of them is redundant.

func _impl_inst() {
  let _grid = withImpl(
    3,
    Impl1.impl, // warning (M0237: omit implicit)
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null), // NO warning (kept)
  );
};
func _impl_inst_ok1() { // implicit omitted, <?R> now pins T — compiles
  let _grid = withImpl(
    3,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _impl_inst_ok2() { // implicit kept, <?R> dropped instead — compiles
  let _grid = withImpl(
    3,
    Impl1.impl,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};
func _impl_inst_error() { // both dropped — error (M0098), nothing pins T
  let _grid = withImpl(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};

func _impl_impl() {
  let _grid = withImpl(
    3,
    Impl1.impl, // warning (M0237: omit outer implicit)
    func _ = withImpl(5, Impl2.impl, func _ = null), // NO warning (inner implicit kept)
  );
};
func _impl_impl_ok1() { // outer implicit omitted, inner kept — compiles
  let _grid = withImpl(
    3,
    func _ = withImpl(5, Impl2.impl, func _ = null),
  );
};
func _impl_impl_ok2() { // outer kept, inner implicit omitted — compiles
  let _grid = withImpl(
    3,
    Impl1.impl,
    func _ = withImpl(5, func _ = null),
  );
};
func _impl_impl_error() { // both omitted — error (M0098)
  let _grid = withImpl(
    3,
    func _ = withImpl(5, func _ = null),
  );
};

func _inst_inst() {
  let _grid = Prim.Array_tabulate<[var ?R]>( // warning (M0223)
    3,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null), // NO warning (kept)
  );
};
func _inst_inst_ok1() { // outer instantiation dropped, inner <?R> kept — compiles
  let _grid = Prim.Array_tabulate(
    3,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _inst_inst_ok2() { // outer kept, inner <?R> dropped — compiles
  let _grid = Prim.Array_tabulate<[var ?R]>(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};
func _inst_inst_error() { // both dropped — error (M0098)
  let _grid = Prim.Array_tabulate(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};

func _inst_impl() {
  let _grid = Prim.Array_tabulate<[var ?R]>( // warning (M0223)
    3,
    func _ = withImpl(5, Impl2.impl, func _ = null), // NO warning (implicit kept)
  );
};
func _inst_impl_ok1() { // outer instantiation dropped, implicit kept — compiles
  let _grid = Prim.Array_tabulate(
    3,
    func _ = withImpl(5, Impl2.impl, func _ = null),
  );
};
func _inst_impl_ok2() { // outer kept, implicit omitted — compiles
  let _grid = Prim.Array_tabulate<[var ?R]>(
    3,
    func _ = withImpl(5, func _ = null),
  );
};
func _inst_impl_error() { // both dropped — error (M0098)
  let _grid = Prim.Array_tabulate(
    3,
    func _ = withImpl(5, func _ = null),
  );
};

// `withImpl` carrying its OWN explicit instantiation on top of the implicit and
// the inner instantiation: three co-dependent removables, T pinned by any one.
func _all_three() {
  let _grid = withImpl<[var ?R]>( // NO warning (kept: pins T for the other two)
    3,
    Impl1.impl, // warning (M0237)
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null), // warning (M0223)
  );
};
func _all_three_error() { // all three dropped — error (M0098)
  let _grid = withImpl(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};

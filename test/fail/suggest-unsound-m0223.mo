//MOC-FLAG -W=M0223,M0236,M0237 --all-libs --package core $MOTOKO_CORE
import List "mo:core/List";
import Array "mo:core/Array";
import Prim "mo:prim";

// 3 seemingly redundant instantiations but at least one is necessary!
func _reports_suggestions(n : Nat) : [List.List<Nat>] {
  Array.tabulate<List.List<Nat>>(
    n,
    func(i) = List.fromArray<Nat>(
      Array.tabulate<Nat>(i + 1, func(j) = j)
    ),
  );
};

// error after dropping all
func _applied_unsound_suggestions(n : Nat) : [List.List<Nat>] {
  Array.tabulate(
    n,
    func(i) = List.fromArray(
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
    Impl1.impl,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _impl_inst_ok1() {
  let _grid = withImpl(
    3,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _impl_inst_ok2() {
  let _grid = withImpl(
    3,
    Impl1.impl,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};
func _impl_inst_error() {
  let _grid = withImpl(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};

func _impl_impl() {
  let _grid = withImpl(
    3,
    Impl1.impl,
    func _ = withImpl(5, Impl2.impl, func _ = null),
  );
};
func _impl_impl_ok1() {
  let _grid = withImpl(
    3,
    func _ = withImpl(5, Impl2.impl, func _ = null),
  );
};
func _impl_impl_ok2() {
  let _grid = withImpl(
    3,
    Impl1.impl,
    func _ = withImpl(5, func _ = null),
  );
};
func _impl_impl_error() {
  let _grid = withImpl(
    3,
    func _ = withImpl(5, func _ = null),
  );
};

func _inst_inst() {
  let _grid = Prim.Array_tabulate<[var ?R]>(
    3,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _inst_inst_ok1() {
  let _grid = Prim.Array_tabulate(
    3,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _inst_inst_ok2() {
  let _grid = Prim.Array_tabulate<[var ?R]>(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};
func _inst_inst_error() {
  let _grid = Prim.Array_tabulate(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};

func _inst_impl() {
  let _grid = Prim.Array_tabulate<[var ?R]>(
    3,
    func _ = withImpl(5, Impl2.impl, func _ = null),
  );
};
func _inst_impl_ok1() {
  let _grid = Prim.Array_tabulate(
    3,
    func _ = withImpl(5, Impl2.impl, func _ = null),
  );
};
func _inst_impl_ok2() {
  let _grid = Prim.Array_tabulate<[var ?R]>(
    3,
    func _ = withImpl(5, func _ = null),
  );
};
func _inst_impl_error() {
  let _grid = Prim.Array_tabulate(
    3,
    func _ = withImpl(5, func _ = null),
  );
};

// `withImpl` carrying its OWN explicit instantiation on top of the implicit and
// the inner instantiation: three co-dependent removables, T pinned by any one.
func _all_three() {
  let _grid = withImpl<[var ?R]>(
    3,
    Impl1.impl,
    func _ = Prim.Array_tabulateVar<?R>(5, func _ = null),
  );
};
func _all_three_error() {
  let _grid = withImpl(
    3,
    func _ = Prim.Array_tabulateVar(5, func _ = null),
  );
};

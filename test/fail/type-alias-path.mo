//MOC-FLAG --package core ../core-stub/src
import MyMap "mo:core/Map";

// Have 2 files with the same import name, but different paths.
import IM1 "type-alias-path/importM1";
import IM2 "type-alias-path/importM2";

module M1 {
  public type Map<K, V> = { map : [(K, [var V])] };
  public func empty<K, V>() : Map<K, V> = { map = [] };
  public func onMap<K, V, T>(k : Map<K, V> -> T) : T = k(empty());
};

module Main {
  func inv<T>(t : T) : T -> () = func(_ : T) = ();
  func check<T>(t : T, _ : T -> ()) {};
  func showRecord(r : {m1 : M1.Map<Nat, Text>; m2 : MyMap.Map<Nat, Text>}) {
    let _ : {m2 : M1.Map<Nat, Text> } = r;
  };
  func showBimatch(r : {m1 : M1.Map<Nat, Text>; m2 : MyMap.Map<Nat, Text>}) {
    check(r.m1, func(m : MyMap.Map<Nat, Text>) = ());
  };
  func showTransitiveImport() {
    let _ : () -> () = MyMap.get // the error message mentions `Types.Order` that is NOT imported here
  };
  func showIM() {
    var r = { x = IM1.m() };
    let r2 = { x = IM2.m() };
    r := r2; // Known issue: `M.MyType` cannot produce `M.MyType`. Since we can freely rename modules, such conflicts are inevitable.
  };
};

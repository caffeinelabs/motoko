//MOC-FLAG --force-gc
import {
  Array_tabulate;
  performanceCounter;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
} = "mo:⛔";

actor Bignum {
  var steps = 0;
  var nat = 3;

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  let arr : [var Nat] = [var 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
  var arr1 : ?[Nat] = null;

  public func go() : async () {
    let (size0, perf0) = counters();

    while (steps < 12) {
      nat := nat * 5 * nat;
      arr[steps] := nat;
      steps := steps + 1;
    };

    let (size1, perf1) = counters();

    debugPrint(debug_show { size = size1 - size0; cycles = perf1 - perf0 });
  };

  public func candid() : async () {
    let (size0, perf0) = counters();
    let ser = to_candid (Array_tabulate<Nat>(arr.size(), func i = arr[i]));
    arr1 := from_candid (ser);
    let (size1, perf1) = counters();
    debugPrint(debug_show { size = size1 - size0; cycles = perf1 - perf0 });
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress candid 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000

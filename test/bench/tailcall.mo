// Benchmark: a Hutton/Bahr-style stack VM running `fak`.
//
// Why this benchmark: the dispatcher is written in *mutual* tail-recursion
// style — `step` matches the opcode and tail-calls a per-opcode handler
// (`opPush`, `opMul`, …); each handler tail-calls `step` again. Today's
// `Tailcall` IR pass (`src/ir_passes/tailcall.ml`) only rewrites *self*
// tail-calls into loops, so each cross-function hop currently allocates a
// fresh frame. Once the optimiser is extended to emit `return_call` for
// general tail calls (TODO at `tailcall.ml:13-14`, unblocked by wasm-exts
// `ReturnCall` landing on this branch), this same dispatcher would lose
// those per-hop frames and the cycle count should drop measurably.
//
// First-order, mutually tail-recursive, no closures — the textbook shape
// where mutual TCO pays off.

import {
  performanceCounter;
  debugPrint;
  trap;
} = "mo:⛔";

persistent actor {

  type Inst = {
    #Push : Int;     // push literal
    #Pop;            // discard top
    #Dup;            // duplicate top
    #Mul;            // pop x, y;  push x * y
    #Sub;            // pop x, y;  push y - x
    #Jz   : Nat;     // pop n; if n == 0 jump to absolute pc
    #Call : Nat;     // call subroutine at absolute pc, pushing ret-pc
    #Ret;            // pop ret-pc and jump there (top-level Ret terminates)
  };

  // Cons-list stacks — keeps the benchmark free of mo:core dependencies.
  type Stk<X> = ?(X, Stk<X>);

  // Hand-coded `fak`, calling itself via the VM's Call/Ret.
  //
  //   fak n = if n == 0 then 1 else n * fak (n - 1)
  //
  //   PC  Instruction       Stack effect
  //   -----------------------------------------------
  //    0  #Dup               [.., n, n]
  //    1  #Jz 8              if n == 0 (consumes a copy) jump to base
  //    2  #Dup               [.., n, n, n]
  //    3  #Push 1            [.., n, n, n, 1]
  //    4  #Sub               [.., n, n, n - 1]
  //    5  #Call 0            recurse; on return: [.., n, fak(n - 1)]
  //    6  #Mul               [.., n * fak(n - 1)]
  //    7  #Ret
  //    8  #Pop               base: discard the leftover 0 → [..]
  //    9  #Push 1            [.., 1]
  //   10  #Ret
  transient let fakCode : [Inst] = [
    #Dup,
    #Jz 8,
    #Dup,
    #Push 1,
    #Sub,
    #Call 0,
    #Mul,
    #Ret,
    #Pop,
    #Push 1,
    #Ret,
  ];

  // ----------------------------------------------------------
  // Mutual-tail-recursion dispatcher.

  func step(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>) : Int =
    switch (code[pc]) {
      case (#Push n) opPush(code, pc, vs, cs, n);
      case (#Pop)    opPop (code, pc, vs, cs);
      case (#Dup)    opDup (code, pc, vs, cs);
      case (#Mul)    opMul (code, pc, vs, cs);
      case (#Sub)    opSub (code, pc, vs, cs);
      case (#Jz t)   opJz  (code, pc, vs, cs, t);
      case (#Call t) opCall(code, pc, vs, cs, t);
      case (#Ret)    opRet (code, pc, vs, cs);
    };

  func opPush(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>, n : Int) : Int =
    step(code, pc + 1, ?(n, vs), cs);

  func opPop(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>) : Int =
    switch vs {
      case (?(_, vs1)) step(code, pc + 1, vs1, cs);
      case null trap "VM: pop on empty stack";
    };

  func opDup(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>) : Int =
    switch vs {
      case (?(x, _)) step(code, pc + 1, ?(x, vs), cs);
      case null trap "VM: dup on empty stack";
    };

  func opMul(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>) : Int =
    switch vs {
      case (?(x, ?(y, vs2))) step(code, pc + 1, ?(x * y, vs2), cs);
      case _ trap "VM: mul underflow";
    };

  func opSub(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>) : Int =
    switch vs {
      case (?(x, ?(y, vs2))) step(code, pc + 1, ?(y - x, vs2), cs);
      case _ trap "VM: sub underflow";
    };

  func opJz(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>, target : Nat) : Int =
    switch vs {
      case (?(n, vs1)) {
        let next = if (n == 0) target else pc + 1;
        step(code, next, vs1, cs);
      };
      case null trap "VM: jz on empty stack";
    };

  func opCall(code : [Inst], pc : Nat, vs : Stk<Int>, cs : Stk<Nat>, target : Nat) : Int =
    step(code, target, vs, ?(pc + 1, cs));

  func opRet(_code : [Inst], _pc : Nat, vs : Stk<Int>, cs : Stk<Nat>) : Int =
    switch cs {
      case (?(retPc, cs1)) step(_code, retPc, vs, cs1);
      case null switch vs {
        case (?(x, _)) x;
        case null trap "VM: top-level Ret with empty value stack";
      };
    };

  func runFak(n : Int) : Int =
    step(fakCode, 0, ?(n, null), null);

  // ----------------------------------------------------------
  // 5-Year-Old Gauss bench: sum [1..100] via naïve self-recursive `foldLeft`.
  //
  // Why this complements the VM bench above: `foldLeft` is *self*-tail-
  // recursive (calls itself with the same type-args), so it hits the
  // existing `Tailcall.transform` loop-rewrite path
  // (`src/ir_passes/tailcall.ml:185-200`) — today the recursion is
  // compiled as a wasm `loop { … local.set; br 0 }`, no actual call
  // frames. Once the loop-rewrite is removed in favour of uniform
  // `return_call` codegen, the cycle delta on this bench is the cost of
  // swapping the `loop` for `return_call $foldLeft`.

  type List = ?(Nat, List);

  func consUp(n : Nat) : List {
    var xs : List = null;
    var i : Nat = 1;
    while (i <= n) {
      xs := ?(i, xs);
      i += 1;
    };
    xs
  };

  func foldLeft<A>(f : (A, Nat) -> A, acc : A, xs : List) : A =
    switch xs {
      case null acc;
      case (?(x, rest)) foldLeft<A>(f, f(acc, x), rest);
    };

  transient let oneToHundred : List = consUp 100;

  // Run `foldLeft (+) 0 [1..100]` 10_000 times. Σ = 5050.
  public func gauss() : async () {
    let n0 = counters();
    var s : Nat = 0;
    var i = 0;
    while (i < 10_000) {
      s := foldLeft<Nat>(func (a, x) = a + x, 0, oneToHundred);
      i += 1;
    };
    let n1 = counters();
    debugPrint(debug_show {
      gauss100 = s;
      iters    = 10_000;
      cycles   = n1 - n0;
    });
  };

  // ----------------------------------------------------------

  func counters() : Nat64 = performanceCounter(0);

  // Run `fak 10` 1_000 times. fak(10) = 3_628_800.
  public func go() : async () {
    let n0 = counters();
    var r : Int = 0;
    var i = 0;
    while (i < 1_000) {
      r := runFak 10;
      i += 1;
    };
    let n1 = counters();
    debugPrint(debug_show {
      fak10  = r;
      iters  = 1_000;
      cycles = n1 - n0;
    });
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress gauss 0x4449444C0000
//MOC-FLAG --experimental-tailcalls

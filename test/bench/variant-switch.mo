// Benchmark: small interpreter for a GHC-Core-like expression language.
// Exercises a 9-arm variant switch (the hot path) heavily.
//
// Constructors:
//   Var, Lit, App, Lam, Let, LetRec, Case, Con, Prim
import {
  performanceCounter;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
  Array_tabulate;
} = "mo:⛔";

persistent actor Core {

  type Expr = {
    #Var    : Text;
    #Lit    : Int;
    #App    : (Expr, Expr);
    #Lam    : (Text, Expr);
    #Let    : (Text, Expr, Expr);     // name, rhs, body
    #LetRec : [(Text, Expr, Expr)];   // list of (name, rhs, body)
    #Case   : (Expr, [(Text, Expr)]); // scrutinee, alts
    #Con    : (Text, [Expr]);         // constructor name, args
    #Prim   : Char;                   // primitive operation
  };

  // Count all nodes in an expression tree
  func size(e : Expr) : Nat =
    switch e {
      case (#Var  _)           1;
      case (#Lit  _)           1;
      case (#App (f, x))       1 + size f + size x;
      case (#Lam (_, b))       1 + size b;
      case (#Let (_, r, b))    1 + size r + size b;
      case (#LetRec triples)   1 + sumTriples triples;
      case (#Case(s, alts))    1 + size s + sumAlts alts;
      case (#Con (_, args))    1 + sumArgs args;
      case (#Prim _)           1;
    };

  func sumTriples(ts : [(Text, Expr, Expr)]) : Nat {
    var n = 0;
    for ((_, r, b) in ts.vals()) n += size r + size b;
    n
  };

  func sumAlts(alts : [(Text, Expr)]) : Nat {
    var n = 0;
    for ((_, e) in alts.vals()) n += size e;
    n
  };

  func sumArgs(args : [Expr]) : Nat {
    var n = 0;
    for (e in args.vals()) n += size e;
    n
  };

  // Build a synthetic expression tree touching all 9 constructors
  func build(d : Nat) : Expr {
    if (d == 0) return #Lit 0;
    let s = build (d - 1 : Nat);
    switch (d % 9) {
      case 0 #App (#Var "x", s);
      case 1 #Lam ("k", s);
      case 2 #App  (s, #Var "y");
      case 3 #Lam  ("z", s);
      case 4 #Let  ("w", s, #Var "w");
      case 5 #LetRec ([("f", s, #App (#Var "f", #Lit 0))]);
      case 6 #Case (s, [("A", #Lit 1), ("B", s)]);
      case 7 #Con  ("Pair", [s, #Var "v"]);
      case _ #App (#Prim '+', s);
    }
  };

  transient let tree = build 15;  // all 9 constructors

  // naïve fib in Core (Peano naturals; #Prim '+' = add, #Prim '-' = pred)
  //   fib 0     = 0
  //   fib (S 0) = 1
  //   fib (S n) = fib n + fib (pred n)
  transient let fibCore : Expr =
    #LetRec ([(
      "fib",
      #Lam ("n",
        #Case (#Var "n", [
          ("0",  #Con ("0", [])),
          ("+1",
            #Case (#App (#Prim '-', #Var "n"), [
              ("0",  #Con ("+1", [#Con ("0", [])])),
              ("+1",
                #Let ("n1", #App (#Prim '-', #Var "n"),
                  #App (
                    #App (#Prim '+',
                      #App (#Var "fib", #Var "n1")),
                    #App (#Var "fib", #App (#Prim '-', #Var "n1")))))
            ]))
        ])),
      #Var "fib"
    )]);

  // ── Weekday: 7-arm variant to compare explicit-arm vs or-pattern dispatch ─
  type Weekday = { #Mon; #Tue; #Wed; #Thu; #Fri; #Sat; #Sun };

  func isWeekend(d : Weekday) : Bool =
    switch d {
      case (#Mon) false;
      case (#Tue) false;
      case (#Wed) false;
      case (#Thu) false;
      case (#Fri) false;
      case (#Sat) true;
      case (#Sun) true;
    };

  func isWeekendOr(d : Weekday) : Bool =
    switch d {
      case (#Mon or #Tue or #Wed or #Thu or #Fri) false;
      case (#Sat or #Sun) true;
    };

  transient let week : [Weekday] =
    [#Mon, #Tue, #Wed, #Thu, #Fri, #Sat, #Sun];

  // ── Runtime values ───────────────────────────────────────────────────────
  type Val = { #VInt : Int; #VFun : Val -> Val; #VCon : (Text, [Val]) };
  type Env = Text -> Val;

  transient let emptyEnv : Env = func(_) { assert false; #VInt 0 };
  func extend(env : Env, x : Text, v : Val) : Env =
    func(y) = if (y == x) v else env y;
  func applyVal(f : Val, v : Val) : Val = switch f {
    case (#VFun g) g v;
    case _         { assert false; #VInt 0 };
  };

  // Peano helpers
  func addPeano(a : Val, b : Val) : Val = switch a {
    case (#VCon (tag, args)) switch tag {
      case "0"  b;
      case "+1" #VCon ("+1", [addPeano (args[0], b)]);
      case _    { assert false; #VInt 0 };
    };
    case _ { assert false; #VInt 0 };
  };
  func predPeano(v : Val) : Val = switch v {
    case (#VCon (_, args)) args[0];
    case _                 { assert false; #VInt 0 };
  };
  func evalPrimOp(c : Char) : Val = switch c {
    case '+' #VFun (func(a) = #VFun (func(b) = addPeano (a, b)));
    case '-' #VFun predPeano;
    case _   { assert false; #VInt 0 };
  };
  func peano(n : Nat) : Val {
    if (n == 0) #VCon ("0", [])
    else        #VCon ("+1", [peano (n - 1 : Nat)])
  };
  func fromPeano(v : Val) : Nat = switch v {
    case (#VCon (tag, args)) switch tag {
      case "0"  0;
      case "+1" 1 + fromPeano (args[0]);
      case _    { assert false; 0 };
    };
    case _ { assert false; 0 };
  };

  // ── Direct AST interpreter ────────────────────────────────────────────────
  func eval(e : Expr, env : Env) : Val = switch e {
    case (#Var x)          env x;
    case (#Lit n)          #VInt n;
    case (#Prim c)         evalPrimOp c;
    case (#App (f, x))     applyVal (eval(f, env), eval(x, env));
    case (#Lam (x, b))     #VFun (func(v) = eval(b, extend(env, x, v)));
    case (#Let (x, r, b))  eval(b, extend(env, x, eval(r, env)));
    case (#LetRec triples) {
      let (x, rhs, body) = triples[0];
      var cell : Val = #VInt 0;
      let recEnv = extend(env, x, #VFun (func(v) = applyVal (cell, v)));
      cell := eval(rhs, recEnv);
      eval(body, recEnv)
    };
    case (#Case (s, alts)) {
      switch (eval(s, env)) {
        case (#VCon (tag, _)) {
          for ((altTag, altBody) in alts.vals()) {
            if (tag == altTag) return eval(altBody, env);
          };
          assert false; #VInt 0
        };
        case _ { assert false; #VInt 0 };
      }
    };
    case (#Con (t, args))
      #VCon (t, Array_tabulate (args.size(), func(i) = eval(args[i], env)));
  };

  // ── Finally-tagless interpreter ───────────────────────────────────────────
  // FT: a compiled term — just a closure Env -> Val, no more variant dispatch
  type FT = Env -> Val;

  type Symantics = {
    lit    : Int -> FT;
    var_   : Text -> FT;
    app    : (FT, FT) -> FT;
    lam    : (Text, FT) -> FT;
    let_   : (Text, FT, FT) -> FT;
    letRec : [(Text, FT, FT)] -> FT;
    case_  : (FT, [(Text, FT)]) -> FT;
    con    : (Text, [FT]) -> FT;
    prim   : Char -> FT;
  };

  transient let evalSem : Symantics = {
    lit    = func(n)        = func(_)   = #VInt n;
    var_   = func(x)        = func(env) = env x;
    app    = func(f, x)     = func(env) = applyVal (f env, x env);
    lam    = func(x, b)     = func(env) = #VFun (func(v) = b (extend(env, x, v)));
    let_   = func(x, r, b)  = func(env) = b (extend(env, x, r env));
    letRec = func(triples)  = func(env) {
      let (x, rhs, body) = triples[0];
      var cell : Val = #VInt 0;
      let recEnv = extend(env, x, #VFun (func(v) = applyVal (cell, v)));
      cell := rhs recEnv;
      body recEnv
    };
    case_  = func(scr, alts) = func(env) {
      switch (scr env) {
        case (#VCon (tag, _)) {
          for ((altTag, altBody) in alts.vals()) {
            if (tag == altTag) return altBody env;
          };
          assert false; #VInt 0
        };
        case _ { assert false; #VInt 0 };
      }
    };
    con  = func(t, args) = func(env) =
      #VCon (t, Array_tabulate (args.size(), func(i) = args[i] env));
    prim = func(c) = func(_) = evalPrimOp c;
  };

  func transform(sem : Symantics, e : Expr) : FT = switch e {
    case (#Var x)          sem.var_ x;
    case (#Lit n)          sem.lit n;
    case (#Prim c)         sem.prim c;
    case (#App (f, x))     sem.app (transform(sem, f), transform(sem, x));
    case (#Lam (x, b))     sem.lam (x, transform(sem, b));
    case (#Let (x, r, b))  sem.let_ (x, transform(sem, r), transform(sem, b));
    case (#LetRec triples) sem.letRec (Array_tabulate (triples.size(), func(i) {
      let (x, r, b) = triples[i]; (x, transform(sem, r), transform(sem, b))
    }));
    case (#Case (s, alts)) sem.case_ (transform(sem, s), Array_tabulate (alts.size(), func(i) {
      let (t, e2) = alts[i]; (t, transform(sem, e2))
    }));
    case (#Con (t, args))
      sem.con (t, Array_tabulate (args.size(), func(i) = transform(sem, args[i])));
  };

  transient let fibFT   : FT  = transform(evalSem, fibCore);
  transient let seven   : Val = peano 7;   // fib(7) = 13

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {
    let (m0, n0) = counters();
    var total = 0;
    var i = 0;
    while (i < 10_000) {
      total += size tree + size fibCore;
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show { total; heap_diff = m1 - m0; instr_diff = n1 - n0 });
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };

  // Benchmark: eval fib(7) via direct AST interpreter vs FT (100 iterations each)
  // Also benchmarks AST→FT transform itself (pure Expr variant dispatch).
  public func evalBench() : async () {
    let fibFn   : Val = eval(fibCore, emptyEnv);  // AST: fib function via eval
    let fibFnFT : Val = fibFT emptyEnv;           // FT:  fib function via compiled form

    let (_m0, n0) = counters();
    var r : Val = seven;
    var i = 0;
    while (i < 100) { r := applyVal (fibFn,   seven); i += 1 };
    let (_m1, n1) = counters();
    var r2 : Val = seven;
    var j = 0;
    while (j < 100) { r2 := applyVal (fibFnFT, seven); j += 1 };
    let (_m2, n2) = counters();
    var xform : FT = fibFT;
    var k = 0;
    while (k < 100) { xform := transform(evalSem, fibCore); k += 1 };
    let (_m3, n3) = counters();
    debugPrint(debug_show {
      fib7_eval      = fromPeano r;
      fib7_evalFT    = fromPeano r2;
      fib7_xform     = fromPeano (applyVal (xform emptyEnv, seven));
      instr_eval      = n1 - n0;
      instr_evalFT    = n2 - n1;
      instr_transform = n3 - n2;
    });
  };
  // Benchmark: isWeekend vs isWeekendOr over all 7 inputs, 10k iterations each.
  // Same dispatch semantics, different source shape — instruction counts should
  // match if or-pattern arms compile to the same br_table path.
  public func weekdayBench() : async () {
    let (_m0, n0) = counters();
    var acc1 = 0;
    var i = 0;
    while (i < 10_000) {
      for (d in week.vals()) { if (isWeekend d) acc1 += 1 };
      i += 1;
    };
    let (_m1, n1) = counters();
    var acc2 = 0;
    var j = 0;
    while (j < 10_000) {
      for (d in week.vals()) { if (isWeekendOr d) acc2 += 1 };
      j += 1;
    };
    let (_m2, n2) = counters();
    debugPrint(debug_show {
      acc1; acc2;
      instr_isWeekend   = n1 - n0;
      instr_isWeekendOr = n2 - n1;
    });
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress evalBench 0x4449444C0000
//CALL ingress weekdayBench 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000

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
  transient let _fibCore : Expr =
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

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {
    let (m0, n0) = counters();
    var total = 0;
    var i = 0;
    while (i < 10_000) {
      total += size tree;
      i += 1;
    };
    let (m1, n1) = counters();
    debugPrint(debug_show { total; heap_diff = m1 - m0; instr_diff = n1 - n0 });
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000

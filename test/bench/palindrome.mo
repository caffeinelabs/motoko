// The 2 palindrome implementations from
// "There and Back Again", by Olivier Danvy and Mayer Goldberg
//
import {
  performanceCounter;
  rts_heap_size;
  debugPrint;
  rts_lifetime_instructions;
} = "mo:⛔";

actor Palindrome {
  type List<A> = ?(A, List<A>);

  func cps(xs : List<Char>) : Bool {
    func walk(xs1 : List<Char>, xs2 : List<Char>, k : List<Char> -> Bool) : Bool = switch (xs1, xs2) {
      case (_, null) { k xs1 }
      case (?(_, xs1), ?(_, null)) { k xs1 }
      case (?(x, xs1), ?(_, ?(_, xs2))) {
        walk(xs1, xs2, func(?(y, ys)) = x == y and k ys);
      }
    };
    walk(xs, xs, func null = true);
  };

  func direct(xs : List<Char>) : Bool {
    func walk(xs1 : List<Char>, xs2 : List<Char>) : ?List<Char> = do ? {
      switch (xs1, xs2) {
        case (_, null) { xs1 } // even length
        case (?(_, xs1), ?(_, null)) { xs1 } // odd length
        case (?(x, xs1), ?(_, ?(_, xs2))) {
          let ?(y, ys) = walk(xs1, xs2)!;
          if x == y {
            ys;
          } else { return null };
        }
      };
    };
    let _ = do ? {
      let null = walk(xs, xs)!;
      return true;
    };
    return false;
  };

  func as_list(cs : Text) : List<Char> {
    var l : List<Char> = null;
    for c in cs.chars() {
      l := ?(c, l);
    };
    l;
  };

  func counters() : (Int, Nat64) = (rts_heap_size(), performanceCounter(0));

  public func go() : async () {

    let (m0, n0) = counters();
    let b = cps(as_list "go hang a salami imalas a gnah og"); // Go hang a salami, i'm a lasagna hog.
    let (m1, n1) = counters();
    debugPrint(debug_show (b, m1 - m0, n1 - n0));

    let (m2, n2) = counters();
    let bn = cps(as_list "go hang a salami Imalas a gnah og"); // Go hang a salami, I'm a lasagna hog.
    let (m3, n3) = counters();
    debugPrint(debug_show (bn, m3 - m2, n3 - n2));

    let (m4, n4) = counters();
    let bp = cps(as_list "go hang a salami imalas a gnah op"); // Go hang a salami, i'm a lasagna hop.
    let (m5, n5) = counters();
    debugPrint(debug_show (bp, m5 - m4, n5 - n4));

    let (i0, j0) = counters();
    let c = direct(as_list "go hang a salami imalas a gnah og");
    let (i1, j1) = counters();
    debugPrint(debug_show (c, i1 - i0, j1 - j0));

    let (i2, j2) = counters();
    let cn = direct(as_list "go hang a salami Imalas a gnah og");
    let (i3, j3) = counters();
    debugPrint(debug_show (cn, i3 - i2, j3 - j2));

    let (i4, j4) = counters();
    let cp = direct(as_list "go hang a salami imalas a gnah op");
    let (i5, j5) = counters();
    debugPrint(debug_show (cp, i5 - i4, j5 - j4));
  };

  public func getPerfData() : async () {
    debugPrint("instructions: " # debug_show (rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000

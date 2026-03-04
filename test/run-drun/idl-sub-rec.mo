//MOC-FLAG -A=M0194
import Prim "mo:⛔";
// test candid subtype check on recursive types
actor this {

  type List<T> = {
    #nil;
    #left: (T, List<T>)
  };

  type Seq<T> = {
    #nil;
    #left: (T, Seq<T>);
    #right: (T, Seq<T>)
  };

  type EvenList<T> = {
    #nil;
    #left:
      (T, {
        //#nil;
        #left: (T, EvenList<T>)})
  };

  type EvenSeq<T> = {
    #nil;
    #left: (T, {
      // #nil;
      #left: (T, EvenSeq<T>);
      #right: (T, EvenSeq<T>)
    });
    #right: (T, {
      // #nil;
      #left: (T, EvenSeq<T>);
      #right: (T, EvenSeq<T>)});
  };

  // sanity subtype checks, verify:
  // List<T> <: Seq<T>
  // EvenList<T> <: List<T>
  // EvenSeq<T> <: Seq<T>
  func sub1<T>(t : List<T>) : Seq<T> { t };
  func sub2<T>(t : EvenList<T>) : List<T> { t };
  func sub3<T>(t : EvenSeq<T>) : Seq<T> { t };

  public func f0(l : Seq<Int>) : async EvenList<Nat> { #nil };

  public func send_f0(
    f : shared EvenList<Nat> -> async Seq<Int>
  ) : async () {
    Prim.debugPrint("ok f0");
  };

  public func send_f1(
    a : [shared EvenList<Nat> -> async Seq<Int>]
  ) : async () {
    Prim.debugPrint("ok f1");
  };

  public func send_f2(
    f : shared List<Nat> -> async EvenSeq<Int>
  ) : async () {
    Prim.debugPrint("ok f2");
  };

  public func send_f3(
    a : [shared List<Nat> -> async EvenSeq<Int>]
  ) : async () {
    Prim.debugPrint("ok f3");
  };

  public func send_f4(
    a : [?(shared List<Nat> -> async EvenSeq<Int>)]
  ) : async () {
    Prim.debugPrint("ok f4");
  };


  func tabulate<T>(n : Nat, v : T) : [T] {
    Prim.Array_tabulate(n, func (_ : Nat) : T { v });
  };

  public func go() : async () {

    let t = debug_show (Prim.principalOfActor(this));

    do {
      try {
        await this.send_f0(f0);
        Prim.debugPrint "ok_0";
      }
      catch e {
        Prim.debugPrint "wrong_0";
      };
    };

    do {
      let this = actor (t) : actor {
        send_f0 : (shared List<Int> -> async Seq<Nat>) -> async ();
      };
      try {
        await this.send_f0(f0);
        Prim.debugPrint "ok_1";
      }
      catch e {
        Prim.debugPrint "wrong_1";
      };
    };


    do {
      let this = actor (t) : actor {
        send_f0 : (shared Seq<Nat> -> async List<Int>) -> async ();
      };
      try {
        await this.send_f0(f0);
        Prim.debugPrint "ok_2";
      }
      catch e {
        Prim.debugPrint "wrong_2";
      };
    };

    do {
      let this = actor (t) : actor {
        send_f0 : (shared Seq<Int> -> async List<Nat>) -> async ();
      };
      try {
        await this.send_f0(f0);
        Prim.debugPrint "ok_3";
      }
      catch e {
        Prim.debugPrint "wrong_3";
      };
    };

    do {
      let this = actor (t) : actor {
        send_f0 : (shared EvenSeq<Int> -> async EvenList<Nat>) -> async ();
      };
      try {
        await this.send_f0(f0);
        Prim.debugPrint "ok_4";
      }
      catch e {
        Prim.debugPrint "wrong_4";
      };
    };

    // negative test
    do {
      let this = actor (t) : actor {
        send_f2 : (shared EvenList<Nat> -> async Seq<Int>) -> async ();
      };
      try {
        await this.send_f2(f0);
        Prim.debugPrint "wrong_5";
      }
      catch e {
        Prim.debugPrint ("ok 5:" # Prim.errorMessage(e))
      };
    };

    // test vectors, should benefit from memoization
     do {
      try {
        await this.send_f1(tabulate(1024, f0));
        Prim.debugPrint "ok_6";
      }
      catch e {
        Prim.debugPrint "wrong_6";
      };
    };

    do {
      let this = actor (t) : actor {
        send_f1 : [shared List<Int> -> async Seq<Nat>] -> async ();
      };
      try {
        await this.send_f1(tabulate(1024, f0));
      }
      catch e {
        Prim.debugPrint "wrong_7";
      };
    };

    do {
      let this = actor (t) : actor {
        send_f1 : [shared Seq<Nat> -> async List<Int>] -> async ();
      };
      try {
        await this.send_f1(tabulate(1024, f0));
        Prim.debugPrint "ok_8";
      }
      catch e {
        Prim.debugPrint "wrong_8";
      };
    };

    // negative test
    do {
      let this = actor (t) : actor {
        send_f3 : [shared EvenList<Nat> -> async Seq<Int>] -> async ();
      };
      try {
        await this.send_f3(tabulate(1024, f0));
        Prim.debugPrint "wrong_9";
      }
      catch e {
        Prim.debugPrint ("ok_9" # Prim.errorMessage(e))
      };
    };

    // test vector defaulting, should benefit from memoization
    do {
      let this = actor (t) : actor {
        send_f4 : [?(shared EvenList<Nat> -> async Seq<Int>)] -> async ();
      };
      try {
        await this.send_f4(tabulate(1024, ?f0));
        Prim.debugPrint ("ok_10")
      }
      catch e {
        Prim.debugPrint ("wrong_10" # Prim.errorMessage(e))
      };
    };


  };
}
//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"

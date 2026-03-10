//MOC-FLAG -A=M0194
import P "mo:⛔";
// test call-raw using candid serialization
actor self {

  public shared func unit() : async () {
    P.debugPrint("unit!");
  };

  public shared func int(n : Int) : async Int {
    P.debugPrint(debug_show("int",n));
    return n;
  };

  public shared func text(t : Text) : async Text {
    P.debugPrint(debug_show("text", t));
    return t;
  };

  public shared func tuple(n: Nat, b: Bool, c: Char) : async (Nat, Bool, Char) {
    P.debugPrint(debug_show("text", (n, b, c)));
    return (n, b, c);
  };

  public shared func trapInt(n : Int) : async Int {
    P.trap("ohoh");
  };

  public shared func supercalifragilisticexpialidocious() : async () {
    P.debugPrint("supercalifragilisticexpialidocious");
  };

  public shared func go() : async () {
    let p = P.principalOfActor(self);

    do {
      let arg = ();
      let res : ?() =
        from_candid(await P.call_raw(p, "unit", to_candid(arg)));
      assert (res == ?arg);
    };

    do {
      let arg : Int = 1;
      let res : ? Int =
        from_candid(await P.call_raw(p,"int", to_candid(arg)));
      assert (res == ?arg);
     };

    do {
      let arg : Text = "hello";
      let res : ?Text =
        from_candid(await P.call_raw(p,"text", to_candid(arg)));
      assert (res == ?arg);
    };

    do {
      let res : ?(Nat, Bool, Char) =
        from_candid(await P.call_raw(p,"tuple", to_candid(1, true, 'a')));
      assert (res == ?(1, true, 'a'));
    };

    do {
      let arg /* : ?(Nat, Bool, Char) */ =  (1, true, 'a');
      try {
        // expected to fail due to arity mismatch
        // (passing a 1 triple where 3 args expected)
        let res : ? (Nat, Bool, Char) =
          from_candid(await P.call_raw(p,"tuple", to_candid(arg)));
        assert false;
      }
      catch e {
        P.debugPrint(P.errorMessage(e));
      }
    };


    do {
      let arg : Int =  1;
      try {
        let res : ? Int =
          from_candid(await P.call_raw(p,"trapInt", to_candid(arg)));
        assert false;
      }
      catch e {
        P.debugPrint(P.errorMessage(e));
      }
    };

    do {
      let m = "super"#"cali"#"fragilisticexpialidocious";
      let res : ?() =
        from_candid(await P.call_raw(p, m, to_candid()));
      assert (res == ?());
    };

  }
};

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress go 0x4449444C0000


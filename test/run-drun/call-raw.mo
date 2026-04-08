//MOC-FLAG -A=M0194
import P "mo:⛔";
import Cycles = "cycles/cycles";

actor self {

  public shared func sint() : async Int {
    return 2;
  };

  public shared func snat() : async Nat {
    return 2;
  };

  public shared func stext() : async Text {
    return "hello";
  };

  public shared func stuple() : async (Nat, Bool, Char) {
    return (1, true, 'a');
  };

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

  public shared func cycles() : async () {
    assert P.cyclesAvailable() == 424242
  };

  public shared func supercalifragilisticexpialidocious() : async () {
    P.debugPrint("supercalifragilisticexpialidocious");
  };

  public shared func go() : async () {
    let p = P.principalOfActor(self);

    do {
      let arg : Blob = "DIDL\00\00";
      let res = await P.call_raw(p, "unit", arg);
      assert res == arg;
    };

    do {
      let arg : Blob = "DIDL\00\01\7c\01";
      let res = await P.call_raw(p, "int", arg);
      assert res == arg;
     };

    do {
      let arg : Blob = "DIDL\00\01\7c\02";
      let res = await P.call_raw(p, "int", arg);
      assert res == arg;
    };

    do {
      let arg : Blob = "DIDL\00\01\71\05\68\65\6c\6c\6f";
      let res = await P.call_raw(p, "text", arg);
      assert res == arg;
    };

    do {
      let arg : Blob = "DIDL\00\03\7d\7e\79\01\01\61\00\00\00";
      let res = await P.call_raw(p, "tuple", arg);
      assert res == arg;
    };

    do {
      let arg : Blob = "DIDL\00\01\7c\01";
      try {
        let res = await P.call_raw(p, "trapInt", arg);
        assert false;
      }
      catch e {
        P.debugPrint(P.errorMessage(e));
      }
    };

    do {
      let m = "super"#"cali"#"fragilisticexpialidocious";
      let arg : Blob = "DIDL\00\00";
      let res = await P.call_raw(p, m, arg);
      assert res == arg;
    };

    do {
      let cycles = 424242;
      let arg : Blob = "DIDL\00\00";
      // legacy way
      Cycles.add<system> cycles;
      let res = await P.call_raw(p, "cycles", arg);
      assert res == arg;
      // new way
      let resu = await (with cycles) P.call_raw(p, "cycles", arg);
      assert resu == arg;
    }
  }
};

//SKIP run
//SKIP run-low
//SKIP run-ir
//CALL ingress sint 0x4449444C0000
//CALL ingress snat 0x4449444C0000
//CALL ingress stext 0x4449444C0000
//CALL ingress stuple 0x4449444C0000
//CALL ingress go 0x4449444C0000


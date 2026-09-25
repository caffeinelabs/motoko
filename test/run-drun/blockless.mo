import { debugPrint; error } = "mo:⛔";

actor A {
  public func foo() : async Int { 42 };

  public func queue(shortCircuit : Bool) : async () {
    debugPrint ("queue test: " # debug_show shortCircuit);
    try await async {
      let a = async ();
      await a;
      ignore async debugPrint "Peek-a-boo!";
      if shortCircuit
        { await? a } // fast await
      else
        { await a }; // proper await
      assert false;
    } catch _ {}
  };

  public func state(shortCircuit : Bool) : async () {
    debugPrint ("state test: " # debug_show shortCircuit);
    var changed = false;
    try await async {
      let a = async ();
      await a;
      changed := true;
      if shortCircuit
        { await? a } // fast await
      else
        { await a }; // proper await
      assert false;
    } catch _ {};
    assert (if shortCircuit { not changed } else { changed });
  };

  public func throwing(shortCircuit : Bool) : async () {
    debugPrint ("throw test: " # debug_show shortCircuit);
    var changed = false;
    try await async {
      let a = async { throw error "Bummer!"; /* issue */ 4578 };
      try ignore await a catch _ { debugPrint "caught it!" };
      changed := true;
      try {
        ignore if shortCircuit
          { await? a } // fast await
        else
          { await a }; // proper await
      } catch _ { debugPrint "caught it again!" };
      assert false;
    } catch _ {};
    assert (if shortCircuit { not changed } else { changed });
  };

  public func go() : async () {
    ignore await? A.foo();
    await A.queue true;
    await A.queue false;
    await A.state false;
    await A.state true;
    await A.throwing false;
    await A.throwing true;
  }
};

A.go() //OR-CALL ingress go 0x4449444C0000

import { error } = "mo:⛔";

actor A {
  func m() : async () {};

  func _t0() : async () {
    try { await m() }
    catch _ {}
    finally { ignore m() } // BAD: no effects allowed!
  };

  func _t1a() : async () {
    try { await m() }
    catch _ {}
    finally { throw error "Nope" } // BAD: has effect.
  };

  func _t1b() : async () {
    try { await m() }
    catch _ {}
    finally { ignore async {} } // BAD: has effect. Weird error...
  };

  func _t2() : async () {
    try { await m() }
    catch _ {}
    finally { 42 } // BAD: should return unit.
  };

  func _t3r() : async () {
    try { await m() }
    catch _ {}
    finally { return } // BAD: no outward edges allowed!
  };

  func _t3l() : async () {
    label out try { await m() }
    catch _ {}
    finally { break out } // BAD: no outward edges allowed!
  };
}

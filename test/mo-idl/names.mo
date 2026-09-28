actor {

  /// named
  public func named(x:Nat, y:Nat) : async (r : Nat) {
    x + y
  };

  /// anon
  public func anon(_ : Nat, _ : Nat) : async Nat {
    0
  };

  /// record
  public func record({ x:Nat; y:Nat }) : async { r:Nat } {
    { r = x + y }
  };

  /// function arg/ret
  public func f(named : shared (x:Nat, y:Nat) -> async (r : Nat)) :
     async (unamed : shared (Nat, Nat) -> async Nat) {
    named
  };

  /// actor arg/ret
  public func g(A :
    actor {
      named : shared (x:Nat, y:Nat) -> async (r : Nat);
      anon : shared (Nat, Nat) -> async Nat
    }) :
  async (actor {
    named : shared (Nat, Nat) -> async Nat;
    anon : shared (x:Nat, y:Nat) -> async (r : Nat)
  }) {
    A
  };

  /// escape candid keywords
  public func escape(int : Int, bool : Bool, service : actor {}) :
   async (int : Int, bool : Bool, service : actor {}) {
    (int, bool, service)
  }

}

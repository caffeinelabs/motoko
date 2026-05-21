import Cycles "mo:core/Cycles";

shared(msg) persistent actor class PiggyBank(
  benefit : shared () -> async (),
  capacity: Nat
  ) {

  transient let owner = msg.caller;

  var savings = 0;

  public shared(msg) func getSavings() : async Nat {
    assert (msg.caller == owner);
    return savings;
  };

  public func deposit() : async () {
    let amount = Cycles.available();
    let limit : Nat = capacity - savings;
    let acceptable =
      if (amount <= limit) amount
      else limit;
    let accepted = Cycles.accept<system>(acceptable);
    assert (accepted == acceptable);
    savings += acceptable;
  };

  public shared(msg) func withdraw(amount : Nat)
    : async () {
    assert (msg.caller == owner);
    assert (amount <= savings);
    await (with cycles = amount) benefit();
    let refund = Cycles.refunded();
    savings -= amount - refund;
  };

}

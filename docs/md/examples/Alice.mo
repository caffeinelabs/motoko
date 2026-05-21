import Cycles = "mo:core/Cycles";
import Lib = "PiggyBank";

persistent actor Alice {

  public func test() : async () {

    let porky = await
      (with cycles = 10_000_000_000_000)
      Lib.PiggyBank(Alice.credit, 1_000_000_000);

    assert (0 == (await porky.getSavings()));

    await (with cycles = 1_000_000) porky.deposit();
    assert (1_000_000 == (await porky.getSavings()));

    await porky.withdraw(500_000);
    assert (500_000 == (await porky.getSavings()));

    await porky.withdraw(500_000);
    assert (0 == (await porky.getSavings()));

    await (with cycles = 2_000_000_000) porky.deposit();
    let refund = Cycles.refunded();
    assert (1_000_000_000 == refund);
    assert (1_000_000_000 == (await porky.getSavings()));

  };

  // Callback for accepting cycles from PiggyBank
  public func credit() : async () {
    let available = Cycles.available();
    let accepted = Cycles.accept<system>(available);
    assert (accepted == available);
  }

}

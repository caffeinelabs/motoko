import Counters "Counters";
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

persistent actor CountToTen {
  public func countToTen() : async () {
    let C : Counters.Counter = await Counters.Counter(1);
    while ((await C.read()) < 10) {
      Debug.print(Nat.toText(await C.read()));
      await C.inc();
    };
  }
};
await CountToTen.countToTen()

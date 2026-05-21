import { print } = "mo:core/Debug";
import { abs } = "mo:core/Int";
import { now } = "mo:core/Time";
import { setTimer; recurringTimer } = "mo:core/Timer";

persistent actor Reminder {

  transient let solarYearSeconds = 356_925_216;

  private func remind() : async () {
    print("Happy New Year!");
  };

  ignore setTimer<system>(#seconds (solarYearSeconds - abs(now() / 1_000_000_000) % solarYearSeconds),
    func () : async () {
      ignore recurringTimer<system>(#seconds solarYearSeconds, remind);
      await remind();
  });
}

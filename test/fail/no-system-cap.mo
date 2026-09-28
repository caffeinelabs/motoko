import { setTimer; cyclesAdd } = "mo:⛔";

actor {

  public query func q() : async () {
    ignore setTimer<system>(1_000_000, false, func () : async () {});
    cyclesAdd<system>(1_000_000);
  };

  public composite query func cq() : async() {
    ignore setTimer<system>(1_000_000, false, func () : async () {});
    cyclesAdd<system>(1_000_000);
  };

}

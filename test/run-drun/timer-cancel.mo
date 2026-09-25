import { debugPrint; setTimer; cancelTimer } = "mo:⛔";

actor {
  var counter = 0;
  var t : Nat = 0;

  private func remind() : async () {
    counter += 1;
    if counter == 3 {
      cancel();
    }
  };

  func cancel() {
    debugPrint("CANCELLING!");
    cancelTimer t;
  };

  let second : Nat64 = 1_000_000_000;

  let max = 4;
  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;

  public shared func go() : async () {
    ignore setTimer<system>(
      2 * second,
      false,
      func () : async () {
        t := setTimer<system>(1 * second, true, remind);
        await remind();
      }
    );

    var attempts = 0;

    while counter < max {
      ignore await raw_rand(); // yield to scheduler
      if counter == 3 { attempts += 1 };
      if attempts >= 200
        { counter += 1 };
    };
    debugPrint(debug_show { attempts; counter });
  };
};

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"

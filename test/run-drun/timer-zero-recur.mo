import { debugPrint; setTimer } = "mo:⛔";

actor {

  // intentionally omit defining `system func timer()`,
  // relying on the compiler filling in a default implementation

  var count = 0;
  let max = 2;
  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;

  public shared func go() : async () {
    var attempts = 0;

    // when duration is 0 all "future" recurrent expirations happen at once
    ignore setTimer<system>(0, true, func () : async () { count += 1; debugPrint "YEP!" });

    while count < max {
      ignore await raw_rand(); // yield to scheduler
      attempts += 1;
      if attempts >= 200 and count == 1
        { count += 1 };
    };
    debugPrint(debug_show { count });
  };
};

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"

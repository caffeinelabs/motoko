//MOC-FLAG -A=M0194
import { debugPrint; error } = "mo:⛔";

actor {

  var count = 0;

  system func timer(setGlobalTimer : Nat64 -> ()) : async () {
    count += 1;
  };

  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;
  public shared func go() : async () {
    var attempts = 0;

    while (count < 1) {
      ignore await raw_rand(); // yield to scheduler
      attempts += 1;
      if (attempts >= 200 and count == 0)
        throw error("he's dead Jim");
    };
    debugPrint(debug_show {count});
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"

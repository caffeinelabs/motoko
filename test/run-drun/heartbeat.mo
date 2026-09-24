import Prim "mo:⛔";

actor {

  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;

  var count = 0;
  let max = 10;

  public shared func inc() : async () {
    if (count < max) {
      count := count + 1
    }
  };

  system func heartbeat() : async () {
    if (count < max) {
      ignore inc();
    }
  };

  public shared func go() : async () {
     var attempts = 0;
     while (count < max) {
       ignore await raw_rand(); // yield to scheduler
       attempts += 1;
       if (attempts >= 200 and count == 0)
         throw Prim.error("he's dead Jim");
     };
     Prim.debugPrint(debug_show {count});
  };
};

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"

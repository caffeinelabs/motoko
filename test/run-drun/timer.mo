//MOC-FLAG -A=M0194
//MOC-ENV MOC_UNLOCK_PRIM=yesplease

import { debugPrint; error; setTimer; cancelTimer } = "mo:⛔";

/*
The schedule is
| seconds | reaction |
|----|-----|
| 1.0 | YEP! |
| 2.0 | DIM! |
| 3.0 | ROOK! |
| 4.0 | BAT! |
| 5.0 | BAT! |
*/

actor {

  system func timer(setGlobalTimer : Nat64 -> ()) : async () {
      await @timer_helper();
  };

  var count = 0;
  var max = 5;
  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;
  let second : Nat64 = 1_000_000_000;

  public shared func go() : async () {
     var attempts = 0;

     var last = 0;
     let id1 = setTimer<system>(1 * second, false, func () : async () { count += 1; debugPrint "YEP!" });
     let id2 = setTimer<system>(2 * second, false, func () : async () { count += 1; debugPrint "DIM!" });
     let id3 = setTimer<system>(3 * second, false, func () : async () {
         count += 1;
         debugPrint "ROOK!";
         last := setTimer<system>(2 * second, true, func () : async () { 
           count += 1; debugPrint "BATT!"; 
           if (count == max) { cancelTimer last; } 
         });
     });

     while (count < max) {
       ignore await raw_rand(); // yield to scheduler
       attempts += 1;
       if (attempts >= 200 and count == 0)
         throw error("he's dead Jim");
     };
     debugPrint(debug_show {count});
  };
};

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL ingress go "DIDL\x00\x00"

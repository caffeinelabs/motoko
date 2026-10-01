import { debugPrint; error; cancelTimer; setTimer } = "mo:⛔";

actor {

  // intentionally omit defining `system func timer()`,
  // relying on the compiler filling in a default implementation

  var count = 0;
  let max = 3;
  let raw_rand = (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand;
  let second : Nat64 = 1_000_000_000;
  var repetition = 0;
  var periodicTimer = 0;

  public shared func go() : async () {
     var attempts = 0;
   
     periodicTimer := setTimer<system>(1 * second, true,
                        func () : async () { 
                          count += 1; 
                          debugPrint ("YEP!");
                          repetition += 1;
                          assert(repetition <= 2);
                          if (repetition == 2) {
                            cancelTimer periodicTimer;
                          }
                        });
     ignore setTimer<system>(1 * second, false,
                            func () : async () { count += 1; debugPrint "EEK!"; assert false });
     ignore setTimer<system>(1 * second, false,
                            func () : async () { count += 1; debugPrint "BEAM!"; throw error("beam me up Scotty!") });

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

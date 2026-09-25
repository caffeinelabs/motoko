// test incremental oom
import P "mo:⛔";
actor {

  public query func go() : async () {
    // allocate 3GB
    var c = 3;
    while c > 0 {
      let a : [var Nat8] = P.Array_init<Nat8>(1024 * 1024 * 1024 / 4, 0xFF);
      c -= 1;
    };

    // allocate 1GB in 1k chunks and trigger Oom
    var d = 1024 * 1024;
    while d > 0 {
      let a : [var Nat8] = P.Array_init<Nat8>(1024 / 4, 0xFF);
      d -= 1;
    };
  }
}

//SKIP run
//SKIP run-low
//SKIP run-ir

//CALL query go "DIDL\x00\x00"

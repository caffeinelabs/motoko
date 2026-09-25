// test incremental oom by allocating 5 GB, one GB at a time!
import P "mo:⛔";
actor {

  var c = 5;

  while c > 0 {
    let a : [var Nat8] = P.Array_init<Nat8>(1024 * 1024 * 1024 / 4, 0xFF);
    c -= 1;
  };

}

//SKIP run
//SKIP run-low
//SKIP run-ir

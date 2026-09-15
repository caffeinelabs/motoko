// test allocation beyond the 32-bit address space
import P "mo:⛔";
do {

  let expectedSize = 5 * 1024 * 1024 * 1024; // 10 GB
  var c = 5;

  while(c > 0) {
    ignore P.Array_init<Nat8>(1024*1024*1024/8, 0xFF);
    c -= 1;
  };

  
  assert(P.rts_memory_size() > expectedSize);
  assert(P.rts_heap_size() > expectedSize);
}

//SKIP run
//SKIP run-low
//SKIP run-ir



// check whether the system capability demand can be satisfied from specific
// `actor` callsites

import { setTimer } = "mo:⛔";

actor {

  system let preupgrade = func () {
    ignore setTimer<system>(0, false, func() : async () {});
  };

  system func postupgrade() {
    ignore setTimer<system>(0, false, func() : async () {});
  };

  ignore setTimer<system>(0, false, func() : async () {}); // from init too
}

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP drun-run
//SKIP wasm-run
//MOC-FLAG -A=M0270

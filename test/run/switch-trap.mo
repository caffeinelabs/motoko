import Prim "mo:⛔";

func byteFrom(seed : Blob) : Nat8 {
  switch seed.values().next() {
    case ?w { w }
    case _ { Prim.trap "Random.byteFrom" }
  }
};
byteFrom("foo");

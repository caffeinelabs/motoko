// The --experimental-stable-memory flag has been removed;
// use of the ExperimentalStableMemory library is an unconditional M0199 error.
import P "mo:⛔";
import {stableMemoryGrow = _} "mo:⛔";
actor {
  let _ = P.stableMemorySize;
}
//SKIP run
//SKIP run-low
//SKIP run-ir

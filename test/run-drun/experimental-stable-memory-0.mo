//MOC-FLAG --experimental-stable-memory 0
import P "mo:⛔";

import {stableMemoryGrow= _} "mo:⛔";
actor {

  transient let _ = P.stableMemorySize;
}

//SKIP run
//SKIP run-low
//SKIP run-ir


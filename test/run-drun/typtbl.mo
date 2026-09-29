//MOC-FLAG -A=M0194 -W=M0145
import { getCandidTypeLimits; setCandidTypeLimits; debugPrint } = "mo:⛔";

actor {
  debugPrint (debug_show (getCandidTypeLimits()));
  setCandidTypeLimits<system> { scalar = 1; bias = 0 };
  debugPrint (debug_show (getCandidTypeLimits()));
  
  transient let ?contents : ?() = from_candid "DIDL\00\00";
  debugPrint "worked";

  setCandidTypeLimits<system> { scalar = 0; bias = 1 };
  debugPrint (debug_show (getCandidTypeLimits()));
  transient let ?_ : ?() = from_candid "DIDL\01\6d\00\00";
  debugPrint "shouldn't appear";
}

//SKIP run
//SKIP run-ir
//SKIP run-low

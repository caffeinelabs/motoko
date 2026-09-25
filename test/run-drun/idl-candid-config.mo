import { setCandidLimits; getCandidLimits } = "mo:⛔";

actor {

  let limits = {
    numerator = 1 : Nat32;
    denominator = 2 : Nat32;
    bias = 3 : Nat32
  };

  setCandidLimits<system>(limits);
  assert getCandidLimits<system>() == limits;

  setCandidLimits<system>(
    {
      numerator = 1;
      denominator = 0; // should trap
      bias = 3
    }
  );
}

//SKIP run-low
//SKIP run
//SKIP run-ir

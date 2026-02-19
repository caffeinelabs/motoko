//MOC-FLAG -A=M0194
// crashes in renaming of or-pattern
// (NB: renaming itself only forced by actor class (to avoid capture when class arg binds parameters)
actor class () {
  type Account = {
          holder : Principal;
          balance : Nat;
  };
  type Amount = {
          amount : Nat
  };
  type Movement = {
          #deposit : Account and Amount;
          #withdraw : Account and Amount;
  };

  func getAccount(m : Movement) : Account =
        switch m {
          case (#deposit accnt or #withdraw accnt) accnt
        }
}

//SKIP run
//SKIP run-ir
//SKIP run-low

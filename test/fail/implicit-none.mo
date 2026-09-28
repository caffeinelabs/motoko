type Order = { #less; #greater; #equal };

func explicit1(n : Nat, m : Nat) : Order { #less };
func cM(n : Nat, m : Nat) : Order { #less };

func cOther(n : Nat, m : Nat) : Order { #less };

module M {

  public func explicit2(n : Nat, m : Nat) : Order { #less };

};

func f1(n : Nat, m : Nat, c : (implicit : (Nat, Nat) -> Order)) {
  ignore c(n, m);
};

f1(0, 1, explicit1); //accept
f1(0, 1);

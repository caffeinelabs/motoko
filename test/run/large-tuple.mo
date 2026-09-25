// This mainly exercises the multi-value-faking code in the backend

func returnsLargeTuple() : (Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20);
func wantsLargeTuple(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20) = ();
wantsLargeTuple(returnsLargeTuple());

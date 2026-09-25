func h<A, B>(f: A -> B) {};
type T = (Int, Int);
func f(x: (Nat, Nat)) : (Int, Int) { x };
func g(x: (Nat, Nat)) : T { x };
h<(Nat, Nat), (Int, Int)>(f); // reject
h<(Nat, Nat), (Int, Int)>(g); // accept

// function arity is syntactic and must match
func t0(f: (Nat, Nat) -> (Nat, Nat))
         : (Nat, Nat) -> (Nat, Nat) = f; // accept

func t1(f: (Nat, Nat) -> (Nat, Nat))
         : (Nat, Nat) -> ((Nat, Nat)) = f; // reject

func t2(f: (Nat, Nat) -> (Nat, Nat))
         : ((Nat, Nat)) -> (Nat, Nat) = f; // reject

func t3(f: (Nat, Nat) -> (Nat, Nat))
         : ((Nat, Nat)) -> ((Nat, Nat)) = f; // reject

func t4(f: (Nat, Nat) -> ((Nat, Nat)))
         : (Nat, Nat) -> (Nat, Nat) = f; // reject

func t5(f: (Nat, Nat) -> ((Nat, Nat)))
         : (Nat, Nat) -> ((Nat, Nat)) = f; // accept

func t6(f: ((Nat, Nat)) -> (Nat, Nat))
         : ((Nat, Nat)) -> (Nat, Nat) = f; // accept

func t7(f: (Nat, Nat) -> ((Nat, Nat)))
         : ((Nat, Nat)) -> ((Nat, Nat)) = f; // reject

func t8(f: ((Nat, Nat)) -> ((Nat, Nat)))
         : ((Nat, Nat)) -> ((Nat, Nat)) = f; // accept

// unary tuples don't exist
func u0(f: Nat -> Nat)
         : Nat -> Nat = f; // accept
func u1(f: Nat -> Nat)
         : (Nat,) -> (Nat,) = f; // accept
func u2(f: Nat -> Nat)
         : (Nat) -> (Nat) = f; // accept

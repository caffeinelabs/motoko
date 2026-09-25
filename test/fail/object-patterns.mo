// Subtyping in patterns

let q : { a : Int; b : Nat } = { a = -42; b = 25 };

func get_a () : Int = switch q {
  case { a = 25 : Nat } { 1 } // NOT OK: Nat cannot consume all Ints
  case { a = 42; b } { b } // OK: 42 is Int by subtyping
  case { a = a : Int; b = 25 : Int } { a } // OK: Int can consume all Nats
};

// the above is analogous to the simpler:

func (a : Int) = switch a {
  case 25 { () } // OK: 25 is Int by subtyping
  case (25 : Int) { () } // OK: 25 is Int by ascription
  case (a : Nat) { () } // NOT OK: Nat cannot consume all Ints
};

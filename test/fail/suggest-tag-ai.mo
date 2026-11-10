//MOC-FLAG --ai-errors
func f(s : { #active; #suspended }) : Nat =
  switch s {
    case (#suspnded) 1;
    case _ 0;
  };

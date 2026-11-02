// A bare record literal cannot be a switch scrutinee — the `{` opens the cases (M0272).
switch { x = 1 } {
  case _ {};
};

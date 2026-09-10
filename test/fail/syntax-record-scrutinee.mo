// A bare record literal cannot be a switch scrutinee — the `{` opens the cases (M0269).
switch { x = 1 } {
  case _ {};
};

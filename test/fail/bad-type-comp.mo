// negative tests for type component syntax
// NB: type components are neither binding nor recursive,
// unlike type definitions in blocks

ignore ((module {}) : module { type T = Null });
ignore ((module { public type T = Int }) : module { type T = Null });
ignore ((module { public type T = Null }) : module { type T = Null; type T = Null });

do {
  type O = {
    type T = ?T // not actually recursive, so reject.
  };
};

do {
  type O = {
    type T = ?U; // not actually mutually recursive, so reject;
    type U = ?T
  };
};

do {
  type F = <A>{ type T = A } -> (); // open type, reject
};

do {
  type F = <A>() -> { type T = A }; // open type, reject
};

do {
  type N = {
    type T = { type U = T; } // not actually recursive, reject.
  };
};

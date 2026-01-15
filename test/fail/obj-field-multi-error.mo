module {
  module A { public type T = () };
  module B { public type T = () };
  type MyType = {
    a : A.MissingA;
    b : B.MissingB;
  };
};

module {
  module A { public type T = () };
  module B { public type T = () };
  type MyTup = (A.MissingA, B.MissingB);
};

module {
  module A { public type T = () };
  module B { public type T = () };
  type MyAnd = A.MissingA and B.MissingB;
};

module {
  module A { public type T = () };
  module B { public type T = () };
  type MyOr = A.MissingA or B.MissingB;
};

module {
  module A { public type T = () };
  module B { public type T = () };
  type MyFunc = (A.MissingA, B.MissingB) -> (A.MissingA, B.MissingB)
};

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
}

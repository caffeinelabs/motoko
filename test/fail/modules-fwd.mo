module X = {

  let C = A.B;

  let u = C.v;

  module A = {
    public module B = {
      public type T = ();
      public let v = ();
    };
  };

};

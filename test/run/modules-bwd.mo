//MOC-FLAG -A=M0194
module X = {

module A = {
  public module B = {
    public type T = ();
    public let v  = ()
  };
};

let C = A.B;

type U = C.T;

let u = C.v;

};



//MOC-FLAG -A=M0194
do {
module X = {
  public type T = Int;
  public let x : T = 1;
};

type T = X.T;
let x = X.x;
let y = X.x + 1;
};


do {
module X = {
  public type T<A> = (A,A);
  public let x : T<Int> = (1, 2);
};

type T<A> = X.T<A>;

let x = X.x;
let (x1, x2) : T<Int> = X.x;
assert (x1 == 1);
assert (x2 == 2);

};


do {
module X = {
  public module X = {
    public type T<A> = (A, A);
    public let x : T<Int> = (1, 2);
  };
};
type T<A> = X.X.T<A>;

let x = X.X.x;
let (x1, x2) : T<Int> = X.X.x;
assert (x1 == 1);
assert (x2 == 2);
};

do {
module X = {
  public module X = {
    public type T<A> = (A, A);
    public let x : T<Int> = (1, 2);
  };
};

module Y = {
  public type T<A> = X.X.T<A>;

  public let x = X.X.x;
  public let (x1, x2) : T<Int> = X.X.x;
};

let _ = assert (Y.x1 == 1);
let _ = assert (Y.x2 == 2);

type U<A> = (X.X.T<A>,Y.T<A>);

};

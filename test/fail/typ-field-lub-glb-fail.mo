class Ctx() {
  public type T = Int;
  public let x = 0;
};

class Ct() {
  public type T = Int;
};

class Cx() {
  public let x = 1;
};

let wrong1 : [Ctx] = [Ctx(), Cx()]; // wrong
let wrong2 : [Ctx] = [Ctx(), Ct()]; // wrong

let ox =
  [
    object { public type T = Int; public let x = 1 },
    object { public type T = Bool; public let x = 2 }
  ][0];

func wrong3() { type U = ox.T }; // wrong, type component mismatch

let px =
  [
    object { public type T = Int; public let x = 1 },
    object { public type T<U> = Int; public let x = 2 }
  ][0];

func wrong4() { type U = px.T }; // wrong, type component arity mismatch

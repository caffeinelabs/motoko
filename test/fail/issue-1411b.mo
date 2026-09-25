module class F() {
  public type T = Int;
  public let x : T = 1;
};

let Z1 : F = F();
type T1 = Z1.T; //ok: can infer type of forward variable reference Z1
let x1  = Z1.x; //ok

let (Z2 : F) = F();
type T2 = Z2.T; //ok: can infer type of forward variable reference Z2
let x2  = Z2.x; //ok

let Z3 = F();
type T3 = Z3.T; //rejected: cannot infer type of forward variable reference Z3
let x3  = Z3.x; //ok

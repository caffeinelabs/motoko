//MOC-FLAG -A=M0194
class Ctx() {
  public type T = Int;
  public let x = 0;
};

class Ctx2() {
  public type T = Int;
  public let x : Int = 0
};

class Cty() {
  public type T = Int;
  public let y = true
};


class Ct() {
  public type T = Int;
};

class Cx() {
  public let x = 1;
};

class Dtx() {
  public type T = Bool;
  public let x = 1
};

let _ : [Ctx] = [ Ctx(), Ctx() ];
let _ : [Ct] = [ Ct(), Ctx() ];
let _ : [Ct] = [ Ctx(), Ct() ];

// let _ : [Ctx] = [ Ctx(), Cx() ]; //wrong
// let _ : [Ctx] = [ Ctx(), Ct() ]; //wrong

let _ : [Cx] = [ Ctx(), Cx() ];
let _ : [Cx] = [ Cx(), Ctx() ];

let _ : [Cx] = [ Dtx(), Cx() ];
let _ : [Cx] = [ Dtx(), Ctx() ];
let _ : Any = [ Dtx(), Ctx() ];

let _ : [Cx] = [ Ctx(), Dtx() ];

let otx : Ctx =
  [ object { public type T = Int; public let x = 1 },
    object { public type T = Int; public let x = 2 } ][0];

func oktx() { type U = otx.T; let x:Nat = otx.x };

let ox =
  [ object { public type T = Int; public let x = 1 },
    object { public type T = Bool; public let x = 2 } ][0];

func okx() { let x : Nat = ox.x };

// func wrong3() { type U = ox.T };

let _ : [ Ctx -> () ] = [
  func (_ : Ctx) {},
  func (_ : Ctx) {}
];

let _ : [ None -> () ] = [
  func (_ : Cx) {},
  func (_ : Ct) {}
];

let _ : [ None -> () ] = [
  func (_ : Ctx) {},
  func (_ : Cty) {}
];

let _ : [ Ctx -> () ] = [
  func (_ : Ctx) {},
  func (_ : Ctx2) {}
];

let _ : [ None -> () ] = [
  func (_ : Dtx) {},
  func (_ : Ctx) {}
];

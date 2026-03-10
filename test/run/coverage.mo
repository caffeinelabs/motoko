//MOC-FLAG -A=M0194
func f() {
  let _ = 0;
  let a = 0;
  let 5 = 0;
  let (5 or 5) = 0;
  let (5 or _) = 0;
  let (_ or 6) = 0;
  let (_ or _) = 0;
  let ?b = ?0;
  let ?_ = ?0;
  let ?9 = ?0;

  func f1(_ : Nat) {};
  func f2(x : Nat) {};
  func f3(5) {};
  //func f4(5 or 5) {};
  func f5(5 or _ : Nat) {};
  func f6(_ or 6 : Nat) {};
  func f7((_ or _) : Nat) {};

  switch 0 { case _ {} };
  switch 0 { case x {} };
  switch 0 { case 5 {} };
  switch 0 { case 5 {}; case 5 {} };
  switch 0 { case 5 {}; case _ {} };
  switch 0 { case 5 {}; case x {} };
  switch 0 { case _ {}; case 6 {} };
  switch 0 { case _ {}; case x {} };
  switch 0 { case x {}; case _ {} };
  switch 0 { case x {}; case x {} };
  switch 0 { case _ {}; case _ {} };
  switch 0 { case (5 or 6) {}; case (7 or 6) {} };
  switch (0, 0) { case (_, _) {}; case (_, 6) {} };
  switch (0, 0) { case (_, (6 or _)) {}; case _ {} };
  switch (0, 0) { case (0, _) {}; case (_, 0) {} };
  switch (0, 0) { case (0, _) {}; case (_, 0) {}; case _ {} };
  switch {a = 0; b = 0} { case {a = _; b = _} {}; case {a = _; b = 6} {} };
  switch {a = 0; b = 0} { case {} {}; case {b = 6} {} };
  switch {a = 0; b = 0} { case {a = _; b = 6 or _} {}; case _ {} };
  switch {a = 0; b = 0} { case {b = 6 or _} {}; case _ {} };
  switch {a = 0; b = 0} { case {a = 0} {}; case {b = 0} {} };
  switch {a = 0; b = 0} { case {a = 0} {}; case {b = 0} {}; case {} {} };
  switch {a = true} { case {a = true} {}; case {a = false} {} };
  switch (#a 6 : {#a : Nat; #b : Nat}) { case (#a _) {}; case (#b _) {} };
  switch (#a 6 : {#a : Nat; #b : Nat}) { case (#a _) {}; case (#b 6) {} };
  switch (#a 6 : {#a : Nat; #b : Nat}) { case (#a _) {}; case (#a 5) {}; case (#b _) {} };
  switch (#a 6 : {#a : Nat; #b : Nat}) { case (#a _) {} };
  switch (#a 6 : {#a : Nat; #b : Nat}) { case (#b _) {} };
  switch (#a 6 : {#a : Nat; #b : Nat; #c}) { case (#b _) {} };
  switch (#a 6 : {#a : Nat; #b : Nat}) {};
  switch (#a 5) { case (#a _) {} };

  func empty() : None = empty();
  switch (empty()) {};
  switch (empty() : {#}) {};
  switch (empty() : {a : Nat; b : None}) {};
  switch (empty() : {a : Nat; b : {#}}) {};
  switch (empty() : {#a : None}) {};
  switch (empty() : {#a : {#}}) {};

  switch (empty()) { case _ {} };
  switch (empty() : {#}) { case _ {} };
  switch (empty() : {a : Nat; b : None}) { case _ {} };
  switch (empty() : {a : Nat; b : {#}}) { case _ {} };
  switch (empty() : {#a : None}) { case _ {} };
  switch (empty() : {#a : {#}}) { case _ {} };
};


type Tree = {#leaf : Int; #branch : (Tree, Tree)};

// Leaf is not fully covered and branch is covered twice
func size(t : Tree) : Nat {
  switch t {
    case (#leaf 3) 1;
    case (#branch(t1, t2)) { 1 + size(t1) + size(t2) };
    case (#branch(t1, t2)) { 1 + size(t1) + size(t2) };
  }
};

// The following is fully covered
func size1(t : Tree) : Nat {
  switch t {
    case (#leaf 3) 1;
    case (#branch(t1, t2)) { 1 + size1(t1) + size1(t2) };
    case (#leaf _) 1;
  }
};

// Example from Sestoft's paper
// "ML pattern match compilation and partial evaluation"
type Lam = {
  #va : Int;
  #lam : (Int, Lam);
  #app : (Lam, Lam);
  #le : (Int, Lam, Lam);
};

func test(t : Lam) : Nat = switch (t) {
  case (#va x) 111;
  case (#lam(x, #va y)) 222;
  case (#lam(x, #lam(y, z))) 333;
  case (#lam(x, #app(y, z))) 444;
  case (#app(#lam(x, y), z)) 555;
  case (#app(#app(x, y), z)) 666;
  case (#le(x, #le(y, z, v), w)) 777;
  case (#lam(x, #le(y, z, v))) 888;
  case (#le(x, y, #app(z, v))) 999;
  case (#app(#app(#lam(x, #lam(y, z)), v), w)) 1010;

  case (#app(_, _)) 2200;
  case (#le(_, _, _)) 2300;
};

assert (test(#le(1, #va 0, #app(#va 0, #va 1))) == 999);


// Patterns redundant through various forms of subtyping

do {
  switch 0 {
    case (-1) {};
    case _ {};
  };
  switch 0 {
    case (-1 : Int) {};
    case _ {};
  };
  switch 0 {
    case (-1 : Int : Nat) {};
    case _ {};
  };
  switch 0 {
    case (-1 : Nat) {};
    case _ {};
  };

  switch null {
    case (?_) {};
    case null {};
  };
  switch null {
    case (?_ : ?Nat) {};
    case null {};
  };
  switch null {
    case (?_ : ?Nat : Null) {};
    case null {};
  };
  switch null {
    case (?_ : Null) {};
    case null {};
  };

  switch (#a) {
    case (#b) {};
    case (#a) {};
  };
  switch (#a) {
    case (#b : {#a; #b}) {};
    case (#a) {};
  };
  switch (#a) {
    case (#b : {#a; #b} : {#a}) {};
    case (#a) {};
  };
  switch (#a) {
    case (#b : {#a}) {};
    case (#a) {};
  };

  let (-1 or _) = 0;
  let (?_ or _) = null;
  let (#a or _) = #b;
};


func h(e : {#}) {
  switch e {
    case (#a) {};
  };
  switch e {
    case (#a : {#a}) {};
  };
  switch e {
    case (#a : {#a} : {#}) {};
  };
  switch e {
    case (#a : {#}) {};
  };

  // Same as above but non-exhaustive
  switch 0 {
    case (-1) {};
  };
  switch 0 {
    case (-1 : Int) {};
  };
  switch 0 {
    case (-1 : Int : Nat) {};
  };
  switch 0 {
    case (-1 : Nat) {};
  };

  switch null {
    case (?_) {};
  };
  switch null {
    case (?_ : ?Nat) {};
  };
  switch null {
    case (?_ : ?Nat : Null) {};
  };
  switch null {
    case (?_ : Null) {};
  };

  switch (#a) {
    case (#b) {};
  };
  switch (#a) {
    case (#b : {#a; #b}) {};
  };
  switch (#a) {
    case (#b : {#a; #b} : {#a}) {};
  };
  switch (#a) {
    case (#b : {#a}) {};
  };

  let -1 = 0;
  let ?_ = null;
  let #a = #b;
};

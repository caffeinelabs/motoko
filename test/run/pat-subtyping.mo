func magic() : None = magic();

if false {

switch (magic ()) {
  case 1 {};  // redundant
  case -1 {};  // redundant
};
switch (magic ()) {
  case (1 : Nat) {};  // redundant
  case (_ : Nat) {};  // redundant
};

switch (magic () : Nat) {
  case (n : Int) {};
};
switch (magic ()) {
  case ((n : Int) : Nat) {};  // redundant
};
switch (magic () : Nat) {
  case (1 : Int) {};
  case (_ : Nat) {};
};
switch (magic ()) {
  case ((1 : Int) : Nat) {};  // redundant
};

switch (magic () : (Nat, Nat)) {
  case (p : (Int, Int)) {};
};
switch (magic ()) {
  case ((p : (Int, Int)) : (Nat, Nat)) {};  // redundant
};
switch (magic () : (Nat, Nat)) {
  case ((x, y) : (Int, Int)) {};
};
switch (magic ()) {
  case (((x, y) : (Int, Int)) : (Nat, Nat)) {};  // redundant
};
switch (magic () : (Nat, Nat)) {
  case (x : Int, y : Int) {};
};
switch (magic ()) {
  case ((x : Int, y : Int) : (Nat, Nat)) {};  // redundant
};
switch (magic () : (Nat, Nat)) {
  case ((1, 2) : (Int, Int)) {};
  case _ {};
};
switch (magic ()) {
  case (((1, 2) : (Int, Int)) : (Nat, Nat)) {};  // redundant
};
switch (magic () : (Nat, Nat)) {
  case (1 : Int, 2 : Int) {};
  case _ {};
};
switch (magic ()) {
  case ((1 : Int, 2 : Int) : (Nat, Nat)) {};  // redundant
};

switch (magic () : {a : Nat; b : Nat}) {
  case (r : {a : Int; b : Int}) {};
};
switch (magic ()) {
  case ((r : {a : Int; b : Int}) : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case ({a; b} : {a : Int; b : Int}) {};
};
switch (magic ()) {
  case (({a; b} : {a : Int; b : Int}) : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case {a = _ : Int; b = _ : Int} {};
};
switch (magic ()) {
  case ({a = _ : Int; b = _ : Int} : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case ({a = 1; b = 2} : {a : Int; b : Int}) {};
  case _ {};
};
switch (magic ()) {
  case (({a = 1; b = 2} : {a : Int; b : Int}) : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case {a = 1 : Int; b = 2 : Int} {};
  case _ {};
};
switch (magic ()) {
  case ({a = 1 : Int; b = 2 : Int} : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case (r : {a : Nat}) {};
};
switch (magic ()) {
  case ((r : {a : Nat}) : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case ({a} : {a : Nat}) {};
};
switch (magic ()) {
  case (({a} : {a : Nat}) : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case (r : {}) {};
};
switch (magic ()) {
  case ((r : {}) : {a : Nat; b : Nat}) {};  // redundant
};
switch (magic () : {a : Nat; b : Nat}) {
  case {} {};
};
switch (magic ()) {
  case ({} : {a : Nat; b : Nat}) {};  // redundant
};

switch (magic ()) {
  case (null) {};  // redundant
  case (? _) {};  // redundant
};
switch (magic () : Null) {
  case (null : ?Nat) {};
};
switch (magic () : ?Nat) {
  case (null) {};
  case (? _) {};
};
switch (magic ()) {
  case ((null : ?Nat) : Null) {};  // redundant
};
switch (magic () : ?Nat) {
  case (?n : ?Int) {};
  case null {};
};
switch (magic ()) {
  case ((?n : ?Int) : ?Nat) {};  // redundant
  case null {};  // redundant
};
switch (magic () : ?Nat) {
  case (?(_ : Int)) {};
  case _ {};
};
switch (magic ()) {
  case (?(_ : Int) : ?Nat) {};  // redundant
};
switch (magic () : ?Nat) {
  case (?1) {};
  case _ {};
};
switch (magic () : ?Nat) {
  case (?1 : ?Int) {};
  case _ {};
};
switch (magic ()) {
  case ((?1 : ?Int) : ?Nat) {};  // redundant
};
switch (magic () : ?Nat) {
  case (?(1 : Int)) {};
  case _ {};
};
switch (magic ()) {
  case (?(1 : Int) : ?Nat) {};  // redundant
};

switch (magic () : {#A : Nat; #B}) {
  case (#A _) {};
  case (#B) {};
};
switch (magic ()) {
  case (#A _ : {#A : Nat; #B}) {};  // redundant
  case (#B : {#A : Nat; #B}) {};  // redundant
};
switch (magic () : {#A : Nat; #B}) {
  case (#A _ : {#A : Nat; #B}) {};
  case (#B : {#A : Nat; #B}) {};
};
switch (magic () : {#A : Nat}) {
  case (#A _ : {#A : Nat; #B}) {};
};
switch (magic ()) {
  case ((#A _ : {#A : Nat; #B}) : {#A : Nat}) {};  // redundant
};
switch (magic () : {#}) {
  case (_ : {#A : Nat; #B}) {};  // redundant
};
switch (magic ()) {
  case ((_ : {#A : Nat; #B}) : {#}) {};  // redundant
};
switch (magic () : {#A : Nat; #B}) {
  case (#A(_ : Int)) {};
  case (#B) {};
};
switch (magic () : {#A : Nat; #B}) {
  case (#A 1) {};
  case _ {};
};
switch (magic () : {#A : Nat; #B}) {
  case (#A(1 : Int)) {};
  case _ {};
};
switch (magic () : {#A : Nat; #B}) {
  case (#A 1 : {#A : Int; #B}) {};
  case _ {};
};

switch (magic ()) {
  case true {};
  case 1 {};
  case 2.5 {};
  case "" {};
  case () {};
  case {} {};
  case null {};
  case (?_) {};
};
};

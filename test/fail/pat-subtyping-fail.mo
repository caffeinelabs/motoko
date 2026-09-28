func magic() : None = magic();

switch (magic () : Nat) {
  case -1 {} // redundant
  case _ {}
};
switch (magic () : Nat) {
  case (-1 : Int) {} // redundant
  case _ {}
};

switch (magic () : (Nat, Nat)) {
  case ((1, -2) : (Int, Int)) {} // redundant
  case _ {}
};
switch (magic () : (Nat, Nat)) {
  case (1 : Int, -2 : Int) {} // redundant
  case _ {}
};

switch (magic () : { a : Nat; b : Nat }) {
  case ({ a = 1; b = -2 } : { a : Int; b : Int }) {} // redundant
  case _ {}
};
switch (magic () : { a : Nat; b : Nat }) {
  case { a = 1 : Int; b = -2 : Int } {} // redundant
  case _ {}
};

switch (magic () : ?Nat) {
  case ? -1 {} // redundant
  case _ {}
};
switch (magic () : ?Nat) {
  case (? -1 : ?Int) {} // redundant
  case _ {}
};
switch (magic () : ?Nat) {
  case ?(-1 : Int) {} // redundant
  case _ {}
};

switch (magic () : { #A : Nat; #B }) {
  case #A(-1) {} // redundant
  case _ {}
};
switch (magic () : { #A : Nat; #B }) {
  case #A(-1 : Int) {} // redundant
  case _ {}
};
switch (magic () : { #A : Nat; #B }) {
  case (#A(-1) : { #A : Int; #B }) {} // redundant
  case _ {}
};

switch (magic () : Int) {
  case (n : Nat) {}
};

switch (magic () : (Int, Int)) {
  case (p : (Nat, Nat)) {}
};
switch (magic () : (Int, Int)) {
  case ((x, y) : (Nat, Nat)) {}
};
switch (magic () : (Int, Int)) {
  case (x : Nat, y : Nat) {}
};

switch (magic () : { a : Int; b : Int }) {
  case (r : { a : Nat; b : Nat }) {}
};
switch (magic () : { a : Int; b : Int }) {
  case ({ a; b } : { a : Nat; b : Nat }) {}
};
switch (magic () : { a : Int; b : Int }) {
  case { a = _ : Nat; b = _ : Nat } {}
};
switch (magic () : { a : Nat }) {
  case (r : { a : Nat; b : Nat }) {}
};
switch (magic () : { a : Nat }) {
  case ({ a; b } : { a : Nat; b : Nat }) {}
};
switch (magic () : {}) {
  case (r : { a : Nat; b : Nat }) {}
};
switch (magic () : {}) {
  case ({ a; b } : { a : Nat; b : Nat }) {}
};

switch (magic () : Null) {
  case null {}
  case ? _ {}
};
switch (magic () : ?Nat) {
  case (null : Null) {}
  case _ {}
};
switch (magic () : ?Int) {
  case (?n : ?Nat) {}
  case null {}
};
switch (magic () : ?Int) {
  case ?(_ : Nat) {}
  case _ {}
};

switch (magic () : { # }) {
  case (#A _ : { #A : Nat; #B }) {} // redundant
};
switch (magic () : { #A : Nat; #B }) {
  case (#A _ : { #A : Nat }) {}
  case _ {}
};
switch (magic () : { #A : Nat; #B }) {
  case (#B : { #B }) {}
  case _ {}
};

switch magic() {
  case true {}
  case 1 {}
};

do {
  func f(x : Int : Nat) {}; // ok
};
do {
  func f(x : Nat : Int) {};
};
do {
  func f(x : Nat : Int : Nat) {};
};
do {
  func f(x : Int : Nat : Int) {};
};
do {
  ignore (func(x : Int) {}) : (Nat -> ()); // ok
};
do {
  ignore (func(x : Nat) {}) : (Int -> ());
};
do {
  ignore (func(x : Nat : Int) {}) : (Nat -> ());
};
do {
  ignore (func(x : Int : Nat) {}) : (Int -> ());
};
do {
  ignore (func f(x : Int) {}) : (Nat -> ()); // ok
};
do {
  ignore (func f(x : Nat) {}) : (Int -> ());
};
do {
  ignore (func f(x : Nat : Int) {}) : (Nat -> ());
};
do {
  ignore (func f(x : Int : Nat) {}) : (Int -> ());
};

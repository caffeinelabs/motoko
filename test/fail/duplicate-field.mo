do { actor class C() {
  public func m() : () {};
  public func n() : () {};
  public func m() : () {};  // Illegal!
}; };

do {
type T = {foo : Int; foo: Bool}
};

do {
type T = {
  foo : Int;
  bar1 : Text;
  bar2 : Text;
  bar3 : Text;
  bar4 : Text;
  bar5 : Text;
  bar6 : Text;
  bar7 : Text;
  foo: Bool;
}
};

do {
type T = {foo : Int; nxnnbkddcv: Bool}
};

do {
type T = {#foo : Int; #nxnnbkddcv: Bool}
};

do {
ignore({foo = 5; foo = true});
};

do {
ignore({foo = 5; nxnnbkddcv = true});
};

do {
ignore(object {public func foo() = (); public func nxnnbkddcv() = ()});
};

do {
ignore(actor {public func foo() : () {}; public func nxnnbkddcv() : () {}});
};

do {
ignore(module {public func foo() = (); public func nxnnbkddcv() = ()});
};

do {
// not a clash
ignore(module {
  public let foo = 1;
  public type foo = Int;
  public type nxnnbkddcv = Nat
});};

do {
// not a clash
// syntax not supported yet
// type T = {type foo = Int; nxnnbkddcv: Bool}
};

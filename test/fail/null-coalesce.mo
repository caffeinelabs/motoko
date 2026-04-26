import Prim "mo:prim";

type MyOpt = ?Nat;
let myOpt : MyOpt = ?5;
let stringOptNull : ?Text = null;
let intOptNull : ?Int = null;
let int : Int = 5;

module Infer {
  func m1() { let _ = Prim.trap("") ?? 42 };
  func m2() { let _ = null ?? 42 };
  func m4<T>(t : T) { let _ = t ?? 42 };
  func m5() { let _ = myOpt ?? 42 };
  func m6() { let _ = intOptNull ?? 42 };
  func m7() { let _ = stringOptNull ?? int };
  func m8() { let _ = 42 ?? 5 };
  func m9() { let _ = "hello" ?? "world" };
  func m10() { let _ = true ?? false };
};

module Check {
  func m1() : Int { Prim.trap("") ?? 42 };
  func m2() : Int { null ?? 42 };
  func m3<T>(t : T) : Int { t ?? 42 };
  func m4() : Int { myOpt ?? 42 };
  func m5() : Int { intOptNull ?? 42 };
  func m51() : Nat { intOptNull ?? 42 };
  func m6() : Any { stringOptNull ?? int };
  func m61() : Text { stringOptNull ?? int };
  func m7() : Nat { 42 ?? 5 };
  func m8<T>(t : T) : Nat { t ?? 42 };
};

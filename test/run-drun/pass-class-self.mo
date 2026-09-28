import { debugPrint; principalOfActor } = "mo:⛔";

actor class C() = Self {
  public func method() : async () { debugPrint "YESS!"; ignore principalOfActor Self };
  func caller(_callee : shared () -> async ()) {};

  debugPrint "Before!";
  ignore principalOfActor Self;
  caller(Self.method);
  caller(method);
  ignore debugPrint (debug_show(principalOfActor Self));
}

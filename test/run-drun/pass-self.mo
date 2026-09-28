import { debugPrint; principalOfActor } = "mo:⛔";

actor Self {
  public func method() : async () { debugPrint "YESS!"; ignore principalOfActor Self };
  func caller(_callee : shared () -> async ()) {};

  debugPrint "Before!";
  ignore principalOfActor Self;
  caller(Self.method);
  caller(method);
  debugPrint (debug_show(principalOfActor Self));
  debugPrint "So far so good!";
}

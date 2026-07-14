import { debugPrint } = "mo:⛔";

// v0: the reference lives in a stable var typed `actor {}`.
actor {
  stable var p : actor {} = actor "aaaaa-aa";

  public func show() : async () {
    // Upcast to Principal (coercion-free identity — keeps the `A` tag) and render.
    debugPrint (debug_show (p : Principal))
  }
}

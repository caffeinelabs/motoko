import { debugPrint; floatToFloat32; float32ToFloat } = "mo:⛔";

actor {
  // scalar: exactly representable in f32
  stable var x : Float32 = 1.5;

  // array: mutable, elements inline in I64 slots
  stable var arr : [var Float32] = [var (1.0 : Float32), (2.0 : Float32), (4.0 : Float32)];

  public func show() : async () {
    debugPrint (debug_show (float32ToFloat x));
    for (v in arr.vals()) debugPrint (debug_show (float32ToFloat v))
  };

  public func mutate() : async () {
    // 3.14 is not exactly representable; f32 rounds it differently from f64,
    // confirming that f32 precision is preserved across the upgrade
    x := (3.14 : Float32);
    arr[1] := (-0.5 : Float32)
  }
}

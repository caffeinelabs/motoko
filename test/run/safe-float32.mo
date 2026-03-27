import { safeFloatToFloat32; floatToFloat32 } = "mo:⛔";

// 1.5 is exactly representable in f32 → round-trip error is zero
assert (safeFloatToFloat32(1.5, 0.0) == ?(floatToFloat32 1.5));

// 0.1: f32 round-trip error ≈ 1.49e-8
assert (safeFloatToFloat32(0.1, 0.0)  == null);   // too tight
assert (safeFloatToFloat32(0.1, 1e-7) != null);   // generous enough

// 3.14: f32 round-trip error ≈ 1.19e-7
assert (safeFloatToFloat32(3.14, 0.0)   == null);
assert (safeFloatToFloat32(3.14, 1e-6)  != null);

// negative epsilon always yields null
assert (safeFloatToFloat32(1.5, -1.0) == null)

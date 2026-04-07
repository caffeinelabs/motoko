import { floatToFloat32; float32ToFloat } = "mo:⛔";

actor {
    // Echo Float32 back (tests input/output Candid serialization)
    public query func echo(x : Float32) : async Float32 { x };
    // Return a Float32 literal (tests literal ascription coercion)
    public query func lit() : async Float32 { 3.14 };
    // Convert Float32 → Float (tests deserialization + NumConvTrapPrim)
    public query func to_f64(x : Float32) : async Float { float32ToFloat x };
}

// echo(1.5) — 1.5 is exactly representable in f32
//CALL query echo 0x4449444c0001730000c03f

// lit() — returns (3.14 : Float32) ≈ 3.1400001049...
//CALL query lit 0x4449444c0000

// to_f64(1.5) — Float32 1.5 promoted to Float64
//CALL query to_f64 0x4449444c0001730000c03f

//SKIP run
//SKIP run-ir
//SKIP run-low

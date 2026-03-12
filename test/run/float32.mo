import { floatToFloat32; float32ToFloat } = "mo:⛔";

let f : Float = 1.5;
let f32 : Float32 = floatToFloat32 f;
let _back : Float = float32ToFloat f32;

//SKIP comp

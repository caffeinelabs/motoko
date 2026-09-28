let _ = 255 : Nat8;
let _ = 65535 : Nat16;
let _ = 4_294_967_295 : Nat32;
let _ = 18_446_744_073_709_551_615 : Nat64;

let _ = 127 : Int8;
let _ = 32767 : Int16;
let _ = 2_147_483_647 : Int32;
let _ = 9_223_372_036_854_775_807 : Int64;

// TODO(gabor) below limits are off-by one, as explained
//             in the open issue in #487.
let _ = -127 : Int8;
let _ = -32767 : Int16;
let _ = -2_147_483_647 : Int32;
let _ = -9_223_372_036_854_775_807 : Int64;

// test patterns

func n8 (n : Nat8) = assert (switch n { case 0 { false } case 1 { false } case 42 { true } case _ { false } });
func n16 (n : Nat16) = assert (switch n { case 0 { false } case 1 { false } case 65000 { true } case _ { false } });
func n32 (n : Nat32) = assert (switch n { case 0 { false } case 1 { false } case 4_294_967_295 { true } case _ { false } });
func n64 (n : Nat64) = assert (switch n { case 0 { false } case 1 { false } case 42 { true } case _ { false } });

n8 42;
n16 65000;
n32 4_294_967_295;
n64 42;

func i8 (n : Int8) = assert (switch n { case 0 { false } case -42 { true } case 1 { false } case 42 { true } case _ { false } });
func i16 (n : Int16) = assert (switch n { case 0 { false } case -32000 { true } case 1 { false } case 32000 { true } case _ { false } });
func i32 (n : Int32) = assert (switch n { case 0 { false } case -20000000 { true } case 1 { false } case 1_294_967_295 { true } case _ { false } });
func i64 (n : Int64) = assert (switch n { case 0 { false } case -420000000000 { true } case 1 { false } case 42 { true } case _ { false } });

i8 42;
i8 (-42);
i16 32000;
i16 (-32000);
i32 1_294_967_295;
i32 (-20000000);
i64 (-420000000000);
i64 42;

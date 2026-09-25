//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";

func serUnit() : Blob = (prim "serialize" : () -> Blob) ();
func deserUnit(x : Blob) : () = (prim "deserialize" : Blob -> ()) x;

func serNats(x: Nat, y: Nat, z: Nat) : Blob = (prim "serialize" : (Nat, Nat, Nat) -> Blob) (x, y, z);
func deserNats(x: Blob) : (Nat, Nat, Nat) = (prim "deserialize" : Blob -> (Nat, Nat, Nat)) x;

func serText(x: Text) : Blob = (prim "serialize" : Text -> Blob) x;
func deserText(x: Blob) : Text = (prim "deserialize" : Blob -> Text) x;

func serBool(x: Bool) : Blob = (prim "serialize" : Bool -> Blob) x;
func deserBool(x: Blob) : Bool = (prim "deserialize" : Blob -> Bool) x;

Prim.debugPrint(debug_show (serUnit ()));
Prim.debugPrint(debug_show (serNats (1, 2, 3)));
Prim.debugPrint(debug_show (serText "Hello World!"));
Prim.debugPrint(debug_show (serBool true));
Prim.debugPrint(debug_show (serBool false));

deserUnit (serUnit ()) : ();
assert ("Hello World!" == deserText (serText "Hello World!"));
// abusing debug_show for easy structural equality
assert(debug_show (1, 2, 3) == debug_show (deserNats (serNats (1, 2, 3)) : (Nat, Nat, Nat)));

assert(true == deserBool (serBool true));
assert(false == deserBool (serBool false));

let arrayNat : [Nat] = [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 67108864, 134217728, 268435456, 536870912, 1073741824, 2147483648, 4294967296, 8589934592, 17179869184, 34359738368];

let arrayInt : [Int] = [-1, -2, -4, -8, -16, -32, -64, -128, -256, -512, -1024, -2048, -4096, -8192, -16384, -32768, -65536, -131072, -262144, -524288, -1048576, -2097152, -4194304, -8388608, -16777216, -33554432, -67108864, -134217728, -268435456, -536870912, -1073741824, -2147483648, -4294967296, -8589934592, -17179869184, -34359738368, -68719476736];

func serArrayNat(a: [Nat]) : Blob = (prim "serialize" : [Nat] -> Blob) a;
func deserArrayNat(b: Blob) : [Nat] = (prim "deserialize" : Blob -> [Nat]) b;

func serArrayInt(a: [Int]) : Blob = (prim "serialize" : [Int] -> Blob) a;
func deserArrayInt(b: Blob) : [Int] = (prim "deserialize" : Blob -> [Int]) b;

let started_with = Prim.rts_heap_size();
assert(arrayNat == deserArrayNat (serArrayNat arrayNat));
assert(arrayNat == deserArrayInt (serArrayNat arrayNat));
assert(arrayNat == deserArrayInt (serArrayInt arrayNat));
assert(arrayInt == deserArrayInt (serArrayInt arrayInt));
let heapDifference = Prim.rts_heap_size() : Int - started_with;

// Difference between enhanced orthogonal persistence and classical persistence (with and without incremental GC).
assert(heapDifference == 2_816 or heapDifference == 4_488 or heapDifference == 4_888);

//SKIP run
//SKIP run-ir
//SKIP run-low

//MOC-FLAG -A=M0194,M0198
import { debugPrint; shiftLeft; shiftRight; nat32ToNat } = "mo:⛔"

func checkShiftLeft(base : Nat, amount : Nat32) =
         assert base * (2 ** nat32ToNat amount) == shiftLeft(base, amount);

checkShiftLeft(42, 7);
checkShiftLeft(42, 24);
checkShiftLeft(42, 25);
checkShiftLeft(42, 26);
checkShiftLeft(42, 25 + 32); // 57
checkShiftLeft(42, 25 + 64); // 89
checkShiftLeft(42, 125);
checkShiftLeft(0, 125);
checkShiftLeft(10 ** 10, 25);

class range(x : Nat32, y : Nat32) {
    var i = x;
    public func next() : ?Nat32 { if (i > y) null else {let j = i; i += 1; ?j} };
};

for (i in range(0, 200)) { checkShiftLeft(1, i) };
for (i in range(0, 200)) { checkShiftLeft(42, i) };

func checkShiftRight(base : Nat, amount : Nat32) =
         assert base / 2 ** nat32ToNat amount == shiftRight(base, amount);

for (i in range(0, 40)) { checkShiftRight(1, i) };
for (i in range(0, 40)) { checkShiftRight(42, i) };

let huge = 2 ** 190;
for (i in range(0, 200)) { checkShiftRight(huge, i) };
for (i in range(0, 200)) { checkShiftRight(huge - 1, i) };

// iterated

assert 1 == shiftRight(shiftRight(huge, 189), 1);
assert 0 == shiftRight(shiftRight(huge, 189), 33);

// roundtrips
for (i in range(0, 200)) { assert 1 == shiftRight(shiftLeft(1, i), i) };
for (i in range(0, 200)) { assert 42 == shiftRight(shiftLeft(42, i), i) };
for (i in range(0, 200)) { assert huge == shiftRight(shiftLeft(huge, i), i) };
for (i in range(0, 200)) { assert huge - 1 == shiftRight(shiftLeft(huge - 1, i), i) }

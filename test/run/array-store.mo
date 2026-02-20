func storeConcreteElementWithBarrier<T>(a : [var Nat], v: Nat) { a[0] := v };
storeConcreteElementWithBarrier([var 1], 123_456_789_000_000);

func storeConcreteElementNoBarrier<T>(a : [var Nat8], v: Nat8) { a[0] := v };
storeConcreteElementNoBarrier<Nat8>([var 1], 1);

func storeGenericElement<T>(a : [var T], v: T) { a[0] := v };
storeGenericElement<Nat>([var 1], 123_456_789_000_000);

func storeInGenericArray<T, A <: [var T]>(a : A, v: T) { a[0] := v };
storeInGenericArray<Nat,[var Nat]>([var 1], 123_456_789_000_000);

type ArrayTypeAlias = [var Nat16];
func storeInAliasTypedArrayNoBarrier<T>(a : ArrayTypeAlias, v: Nat16) { a[0] := v };
storeInAliasTypedArrayNoBarrier<Nat16>([var 1], 1);

type ElementTypeAlias = Int8;
func storeAliasTypedElementNoBarrier(a : [var ElementTypeAlias], v: Int8) { a[0] := v };
storeAliasTypedElementNoBarrier([var 1], 1);

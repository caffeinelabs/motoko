assert ((0x5bafecbd : Nat32) <>> (4 : Nat32) == (0xd5bafecb : Nat32));
assert ((0x5bafecbd : Nat32) <>> (36 : Nat32) == (0xd5bafecb : Nat32));

assert ((0x5bbd : Nat16) <>> (4 : Nat16) == (0xd5bb : Nat16));
assert ((0x5bbd : Nat16) <>> (20 : Nat16) == (0xd5bb : Nat16));

assert ((0x56 : Nat8) <>> (3 : Nat8) == (0xca : Nat8)); // 01010110 -> 11001010
assert ((0x56 : Nat8) <>> (11 : Nat8) == (0xca : Nat8));
assert ((0x56 : Nat8) <>> (19 : Nat8) == (0xca : Nat8));

assert ((0x5bafecbd : Nat32) <<> (4 : Nat32) == (0xbafecbd5 : Nat32));
assert ((0x5bafecbd : Nat32) <<> (36 : Nat32) == (0xbafecbd5 : Nat32));

assert ((0x5bbd : Nat16) <<> (4 : Nat16) == (0xbbd5 : Nat16));
assert ((0x5bbd : Nat16) <<> (20 : Nat16) == (0xbbd5 : Nat16));

assert ((0x56 : Nat8) <<> (3 : Nat8) == (0xb2 : Nat8)); // 01010110 -> 10110010
assert ((0x56 : Nat8) <<> (11 : Nat8) == (0xb2 : Nat8));
assert ((0x56 : Nat8) <<> (19 : Nat8) == (0xb2 : Nat8));

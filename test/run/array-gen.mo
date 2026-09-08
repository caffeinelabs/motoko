import Prim "mo:⛔";

let a = Prim.Array_init<Nat>(10,42);

assert (a.size() == 10);
for (n in a.vals()) {
  assert(n == 42);
};
for (n in a.values()) {
  assert(n == 42);
};

let b = Prim.Array_tabulate<Nat>(10,func (x : Nat) : Nat = x);

assert(b.size() == 10);
for (i in b.keys()) {
  assert (b[i] == i);
};

let c = Prim.Array_tabulateVar<Nat>(10,func (x : Nat) : Nat = x);

for (i in c.keys()) {
  c[i] += 1;
};

assert(c.size() == 10);
for (i in c.keys()) {
  assert (c[i] == i + 1);
};

func init(length: Nat) {
    let array1 = Prim.Array_init<Nat>(length, 42);
    assert array1.size() == length;
    var index = 0;
    while (index < length) {
        assert array1[index] == 42;
        index += 1;
    }
};

init(0);
init(1);
init(2);
init(1000);
init(1000_000);

func tabulate(length: Nat) {
    let array2 = Prim.Array_tabulate<Nat>(length, func i { 1 + 2 * i });
    assert array2.size() == length;
    var index = 0;
    while (index < length) {
        assert array2[index] == 1 + 2 * index;
        index += 1
    }
};

tabulate(0);
tabulate(1);
tabulate(2);
tabulate(1000);
tabulate(1000_000);
//MOC-FLAG -A=M0269

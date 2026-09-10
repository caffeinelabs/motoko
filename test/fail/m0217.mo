// Verifies that explicit `persistent` on an actor emits M0217.
persistent actor class C1() = this {
  let _x = 1;
};

persistent actor class C2() = this {
  let _y = 2;
};
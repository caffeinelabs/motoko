// Verifies that explicit `persistent` on an actor emits M0217.
persistent actor {
  let _x = 1;
}

persistent actor class C() = this {
  let _y = 2;
};
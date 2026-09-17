// Verifies that explicit `stable` on an actor field emits M0218.
actor {
  stable let _x = 1;
  stable var _y = 2;
};

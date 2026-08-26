import M "mixins/Plain";

// The reference interpreter cannot import mixins (separate, pre-existing
// limitation); the runner executes this test on drun, which compiles to wasm.
//SKIP run
//SKIP run-ir
//SKIP run-low

persistent actor {
  // `system` is redundant: `Plain` does not require it (warning M0265).
  // Guards against regressions where `moc --check` accepts the program but
  // `moc -c` crashes on untyped mixin include content.
  include M<system>();

  public func check() : async Text {
    await mixinGreet()
  };
};

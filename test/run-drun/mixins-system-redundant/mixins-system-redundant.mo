import M "mixins/Plain";

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

import A "mixin-collision/a/Mix";
import B "mixin-collision/b/Mix";
actor {
  include A();
  include B();
};

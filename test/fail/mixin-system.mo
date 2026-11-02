import MixinSystem "mixins/System";
import MixinSystem2 "mixins/System2";

actor {
  include MixinSystem<system>(); // Fine
  include MixinSystem2(); // Fails
};

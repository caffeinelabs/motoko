import MixinCounter "mixins/Counter";

persistent actor {
  include MixinCounter<system>();
};

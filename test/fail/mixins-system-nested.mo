import Nested "mixins/NestedSystem";

actor {
  include Nested<system>(); // Fine
};

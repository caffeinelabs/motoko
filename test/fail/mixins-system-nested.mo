import Nested "mixins/NestedSystem";

persistent actor {
  include Nested<system>(); // Fine
};

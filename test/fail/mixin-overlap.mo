import CounterMixin "mixins/Counter";

actor {
  include CounterMixin(0);
  public func increment() : () { };
};

actor {
  public func increment() : () { };
  include CounterMixin(0);
};

actor {
  type T = Int;
  include CounterMixin(0);
};

actor {
  include CounterMixin(0);
  type T = Int;
};

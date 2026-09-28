import CounterMixin "mixins/Counter";
// checks this has all fields
actor this {
  include CounterMixin(0);
  public func decrement() : () { };
  do {
    let _ = this.increment;
    let _ = this.decrement;
  };
};

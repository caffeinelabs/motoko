actor Counter {
  var value = 0;
  public func inc() : async Nat {
    value += 1;
    return value;
  };
};

actor Factorial {

  var last = 1;

  public func next() : async Nat {
    last *= await Counter.inc();
    return last;
  }
};

ignore await Factorial.next();
ignore await Factorial.next();
await Factorial.next();

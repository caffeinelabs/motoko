class range(x : Nat, y : Nat) {
  var i = x;
  public func next() : ?Nat { if (i > y) null else {let j = i; i += 1; ?j} };
};

func wrong() { for (i in range(0,1)) ignore 666 };
wrong();

 (let _ = 1) : ()  ;



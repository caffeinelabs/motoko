persistent actor Atomicity {

  transient var s = 0;
  transient var pinged = false;

  public func ping() : async () {
    pinged := true;
  };

  // an atomic method
  public func atomic() : async () {
    s := 1;
    ignore ping();
    ignore 0/0; // trap!
  };

  // a non-atomic method
  public func nonAtomic() : async () {
    s := 1;
    let f = ping(); // this will not be rolled back!
    s := 2;
    await f;
    s := 3; // this will not be rolled back!
    await f;
    ignore 0/0; // trap!
  };

};

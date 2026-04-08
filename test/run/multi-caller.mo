

actor a {

  // returns caller id
  public shared(c) func getCaller() : async Principal {
    c.caller
  };

  // returns self id when called (internally or externally)
  public shared func getSelf()  : async Principal {
    await getCaller();
  };

};


actor class C () {

  // returns caller id
  public shared(c) func getCaller()  : async Principal {
    c.caller
  };

  // returns self id when called (internally or externally)
  public shared func getSelf()  : async Principal {
    await getCaller();
  };

};

let alias = a;
let b = await C();
let c = await C();

ignore async {
  let id_a = await a.getSelf();
  let id_b = await b.getSelf();
  let id_c = await c.getSelf();
  let id_alias = await alias.getSelf();
  // check ids are distinct
  assert (id_a != id_b);
  assert (id_b != id_c);
  assert (id_c != id_a);
  assert (id_alias == id_a);
};


// test caller alternation is correct

actor Ping {

  // returns caller id
  public shared(c) func getCaller()  : async Principal {
    c.caller
  };

  // returns self id when called (internally or externally)
  public func getSelf() : async Principal {
    await getCaller();
  };

  public shared(c) func call (n:Nat) : async () {
    if (n > 0) {
      assert (c.caller == (await Pong.getSelf()));
      await Pong.call(n - 1);
    };
  };
};

actor Pong {

  // returns caller id
  public shared(c) func getCaller()  : async Principal {
    c.caller
  };

  // returns self id when called (internally or externally)
  public func getSelf()  : async Principal {
    await getCaller();
  };

 public shared(c) func call (n:Nat) : async () {
    if (n > 0) {
      assert c.caller == (await Ping.getSelf());
      await Ping.call(n - 1);
    };
 };

 public func test(n:Nat) : () {
    ignore async await Ping.call(n);
 };

};

Pong.test(5);

//no support for multiple-toplevel actors and await
//SKIP comp

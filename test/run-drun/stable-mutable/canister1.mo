actor {

  flexible let array = [var 1, 2, 3];
  flexible let obj = { var field = "hello"; extra = 1 };

  let a = (array, array);
  let b = (obj : { var field : Text }, obj);

  type Cyclic = { var self : [Cyclic]; var field : Text };
  let c : Cyclic = { var self = ([] : [Cyclic]); var field = "hello"; };

  public func tie() : async () {
    c.self := [c];
  };

  public query func checkArray() : async () {
    // check that mutable values are properly aliased

    for i in a.0.keys() {
      assert(a.0[i] == a.1[i]);
      a.0[i] += 1;
      assert(a.0[i] == a.1[i]);
    };
  };

  public query func checkRecord() : async () {

    assert(b.0.field == "hello");
    assert(b.1.field == "hello");
    b.0.field #= "!";
    assert(b.0.field == b.1.field);

    // Check that the extra field is still there
    assert(b.1.extra == 1);

  };

  public query func checkCycle() : async () {
    assert(c.self.size() == 1);
    assert(c.field == "hello");
    assert(c.self[0].field == "hello");
    c.field #= "!";
    assert(c.field == c.self[0].field);
  };
}

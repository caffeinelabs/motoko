module {

  type Cyclic = { var self : [Cyclic]; var field : Text };

  public func migration({
    a : ([var Nat], [var Nat]);
    b : (
      { var field : Text },
      { var field : Text; extra : Nat }
    );
    c : Cyclic
  }) : {
    a : (Any, [var Nat]);
    b : (
      Any,
      { var field : Text; extra : Nat }
    )
  } = {
    a = a;
    b = b;
  }

}

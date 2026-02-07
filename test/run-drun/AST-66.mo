// test cps conversion of async blocks with type decs
actor a {
  public func go() : (){
    ignore async{
      type T = Null;
      await { async (null:T) };
    };

    ignore async{
      type T = U;
      let _ = await { async (null:T) };
      type U = Null;
      await { async (null:T) };
    };
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"


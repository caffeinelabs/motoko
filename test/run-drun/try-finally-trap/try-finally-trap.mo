import { debugPrint; rts_callback_table_count } =  "mo:⛔";

actor A {

  public func go() : async () {
    try {
      await async ();
      assert false
    }
    finally {
      debugPrint("trap in finally!");
      assert false
    };
  };

  public func show() : async () {
    debugPrint(debug_show
      { rts_callback_table_count = rts_callback_table_count() });
  };

};

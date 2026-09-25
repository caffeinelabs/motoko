import { debugPrint; rts_callback_table_count } =  "mo:⛔";

actor class C() {

  // leak a callback_table_slot by trapping in finally
  public func leak() : async () {
    try {
      await async ();
    }
    finally {
      debugPrint("trap in finally!");
      assert false
    };
  };

  public func show() : async () {
    debugPrint(debug_show
      {
        rts_callback_table_count = rts_callback_table_count()
      });
  };

};

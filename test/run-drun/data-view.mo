// simplified version of data-view.mo that doesn't require core.
//MOC-FLAG --generate-view-queries
import Prim "mo:⛔";
persistent actor Self {

  module ArrayView {
    public func view<V>(self : [var V]) :
      (start : Nat, count : Nat) -> [V] =
      func (start, count) {
        Prim.Array_tabulate<V>(count, func i { self[start+i] });
      }
  };

  let array : [var (Nat, Text)] = [var (1, "1"), (2,"2")];

  /* generates */
  public query func __array(start:Nat, count: Nat) : async [(Nat, Text)] {
     array.view()(start, count);
  };

  // here, [insible_array.view] produces a non-shared (mutable) type, omit viewer
  // later maybe approximate by shared type.
  let invisible_array : [[var Nat]] = [];

  // shared values we can just display, sans viewer
  type Tree = { #leaf; #node : (Tree, Nat, Tree) };
  var some_variant = #node (#leaf, 0, #leaf);
  let some_record = {a=1;b ="hello"; c = true} ;

  // stable, non-shared values we can't just display in full, without viewer
  let some_mutable_record = {var a = 1};

  public query func __override(): async Text { "user defined __override" };

  let override = #override;
  /* generates nothing as would clash with user-defined __override above" */

  let motoko_xxx = #motoko_xxx;
  /* generates nothing as would clash with reserved __motoko_ members" */

  public func go() : async () {
    let views = actor (debug_show (Prim.principalOfActor(Self))) :
      actor {
        /* generated */
        __array : shared query (Nat, Nat) -> async [(Nat, Text)];
        __some_variant: shared query () -> async Tree;
	__some_record : shared query () -> async {a:Nat; b: Text; c : Bool};
	/* user-defined */
	__override : shared query () -> async Text;
	/* unable to generate because of (potential) name clash, no viewer or non-shared type*/
        __invisible_array : shared query() -> async None;
        __some_mutable_record : shared query() -> async None;
        __motoko_xxx : shared query () -> async None;

    };
    Prim.debugPrint(debug_show (await views.__array(0,0)));
    Prim.debugPrint(debug_show (await views.__some_variant()));
    Prim.debugPrint(debug_show (await views.__some_record()));
    Prim.debugPrint(debug_show (await views.__override())); // calls user-defined method
    try {
      await views.__invisible_array(); //fails with method not available
      assert false;
    } catch (e) {
      Prim.debugPrint (Prim.errorMessage(e));
    };
    try {
      await views.__some_mutable_record(); //fails with method not available
      assert false;
    } catch (e) {
      Prim.debugPrint (Prim.errorMessage(e));
    };
    try {
      await views.__motoko_xxx(); //fails with method not available
      assert false;
    } catch (e) {
      Prim.debugPrint (Prim.errorMessage(e));
    }

  }

}
//CALL ingress go "DIDL\x00\x00"

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

  /* generates
  public query func __array(start:Nat, count: Nat) : async [(Nat, Text)] {
     array.view()(start, count);
  };
  */

  // here, [array_of_non_shared.view] produces a non-shared result type,
  // approximated to Any entries
  let array_of_non_shared : [var { var a : {#A}; b : {#B}; c : [var {#C}]}] =
     [ var {var a = #A; b = #B; c = [var #C]} ];

  /* generates
  public query func __array_of_non_shared() :
    async Any // <- approximation
  {
     array_of_non_shared : Any;
  };
  */

  // here, [non_shared_array.view] produces a non-shared (mutable) type,
  // approximate to Any
  let non_shared_array : [[var Nat]] = [];


  // shared values we can just display, sans viewer
  type Tree = { #leaf; #node : (Tree, Nat, Tree) };
  var some_variant = #node (#leaf, 0, #leaf);
  let some_record = {a=1;b ="hello"; c = true} ;

  // stable, non-shared values we can't just display in full, without viewer
  // approximate to shared supertype { b : Nat; c : Any},
  // dropping mutable fields, promoting non-shared to Any
  let some_mutable_record = {var a = 1; b = 0; c = [var 0]};

  //recursive types:
  type List<T> = ?(T, List<T>);

  // all shared
  let some_list : List<Nat> = ?(1,?(2,null));

  // non-shared, approximated
  let some_non_shared_list : List<{var a : Nat; b : Text}> =
    ?({var a = 1; b = "1"},
      ?({var a = 2; b = "2"}, null));

  public query func __override(): async Text { "user defined __override" };

  let override = #override;
  /* generates nothing as would clash with user-defined __override above" */

  let motoko_xxx = #motoko_xxx;
  /* generates nothing as would clash with reserved __motoko_ members" */


  public func go() : async () {
    let views = actor (debug_show (Prim.principalOfActor(Self))) :
      actor {
        /* generated .views */
        __array : shared query (Nat, Nat) -> async [(Nat, Text)];
        __array_of_non_shared : shared query () -> async Any; // approximation
	/* generated simple */
        __some_variant: shared query () -> async Tree;
	__some_record : shared query () -> async {a:Nat; b: Text; c : Bool};
	/* user-defined */
	__override : shared query () -> async Text;
	/* unable to generate because of (potential) name clash, no viewer or non-shared type*/
        __non_shared_array : shared query() -> async Any; // approximation
        __some_mutable_record : shared query() -> async Any; // approximation
	__some_list : shared query () -> async List<Nat>;
	__some_non_shared_list : shared query () -> async Any; // approximation
        __motoko_xxx : shared query () -> async None;

    };
    func printAny(_ : Any) { Prim.debugPrint("any") };
    Prim.debugPrint(debug_show (await views.__array(0,0)));
    Prim.debugPrint(debug_show (await views.__some_variant()));
    Prim.debugPrint(debug_show (await views.__some_record()));
    Prim.debugPrint(debug_show (await views.__override())); // calls user-defined method
    Prim.debugPrint(debug_show (await views.__some_list()));
    // debug_show doesn't support Any values, so avoid those below
    printAny(await views.__array_of_non_shared());
    printAny(await views.__non_shared_array());
    printAny(await views.__some_mutable_record());
    printAny(await views.__some_non_shared_list());
    try {
      await views.__motoko_xxx(); //fails with method not available
      assert false;
    } catch (e) {
      Prim.debugPrint (Prim.errorMessage(e));
    }

  }

}
//CALL ingress go "DIDL\x00\x00"

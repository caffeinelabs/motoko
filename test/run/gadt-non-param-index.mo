// GADT indexed by a concrete tag type (not a refined outer type
// variable). Same singleton-variant trick as gadt-phantom-state.mo
// so the tags are distinct cons that refinement can discriminate.

type NatTag  = { #natTag };
type TextTag = { #textTag };
type BoolTag = { #boolTag };

type Val<T> = {
  #natV  : type T = NatTag  in Nat;
  #textV : type T = TextTag in Text;
  #boolV : type T = BoolTag in Bool;
};

func describe<T>(v : Val<T>) : Text =
  switch v {
    case (#natV  n) "nat:"  # debug_show n;
    case (#textV t) "text:" # t;
    case (#boolV b) "bool:" # debug_show b;
  };

let v1 : Val<NatTag>  = #natV 42;
let v2 : Val<TextTag> = #textV "hello";
let v3 : Val<BoolTag> = #boolV true;

assert describe v1 == "nat:42";
assert describe v2 == "text:hello";
assert describe v3 == "bool:true";

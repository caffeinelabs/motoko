// End-to-end test for structural implicit derivation over variants (unary).
// __variant triggers synthesis: the wrapper switches on the active case and
// passes (tag, lazy payload thunk) to the combiner.
//MOC-FLAG --package core $MOTOKO_CORE -W=M0223,M0236,M0237

// Structural combiner: serialise a variant as "#tag(payload)".
// __variant triggers structural synthesis; elem_typ = Text.
func show(__variant : (Text, () -> Text)) : Text {
  let (tag, payload) = __variant;
  "#" # tag # "(" # payload() # ")";
};

// Per-payload instances (each returns Text = elem_typ)
module TextShow { public func show(self : Text) : Text = self };
module NatShow { public func show(self : Nat) : Text = debug_show self };
module BoolShow { public func show(self : Bool) : Text = if self "true" else "false" };
// Instance for the unit payload of no-arg cases like `#red`
module UnitShow { public func show(_self : ()) : Text = "" };

// Entry point: implicit `show : T -> Text`
func inspect<T>(x : T, show : (implicit : T -> Text)) : Text = show(x);

// Single-case variant
let s1 = inspect(#circle (5 : Nat));
assert s1 == "#circle(5)";

// Multi-case variant: the wrapper covers every case; only the active one fires.
type Shape = { #circle : Nat; #flag : Bool; #named : Text };
let s2 = inspect<Shape>(#named "hi");
assert s2 == "#named(hi)";
let s3 = inspect<Shape>(#flag true);
assert s3 == "#flag(true)";

// Enum-style variant: every payload is unit, resolved via UnitShow.
type Color = { #red; #green; #blue };
let s4 = inspect<Color>(#green);
assert s4 == "#green()";

// Nested: a variant whose payload is itself a record, derived recursively.
module RecordShow {
  public func show(__record : [(Text, () -> Text)]) : Text {
    var s = "{";
    var first = true;
    for ((k, v) in __record.vals()) {
      if (not first) { s #= "," };
      s #= k # "=" # v();
      first := false;
    };
    s # "}";
  };
};
type Event = { #login : { user : Text }; #ping : Nat };
let s5 = inspect<Event>(#login { user = "alice" });
assert s5 == "#login({user=alice})";
let s6 = inspect<Event>(#ping 7);
assert s6 == "#ping(7)";

// Recursive variant: the `#nest : Tree` case re-needs `show : Tree -> Text`,
// resolved to the in-progress wrapper via the recursion-cycle mechanism.
type Tree = { #leaf : Nat; #nest : Tree };
let s7 = inspect<Tree>(#nest (#nest (#leaf 3)));
assert s7 == "#nest(#nest(#leaf(3)))";

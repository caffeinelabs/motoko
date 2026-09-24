//MOC-FLAG -W=M0237 --error-format=json
// The M0237 edit for an implicit argument in last position must take the
// argument's trailing comma along, and must not delete a juxtaposed sole
// argument outright: either would leave a syntax error once applied.

module NatText {
  public func toText(_ : Nat) : Text = "";
};

func payload<V>(v : V, toText : (implicit : V -> Text)) : Text = toText v;
func only<V>(toText : (implicit : V -> Text)) : ?V { ignore toText; null };

// Edit must remove `NatText.toText,\n` (up to the `)`), not leave `,\n,`.
ignore payload(
  1 : Nat,
  NatText.toText,
);

// Edit must turn `only<Nat> toText` into `only<Nat> ()`, not `only<Nat> `.
do {
  let toText = NatText.toText;
  ignore only<Nat> toText;
};

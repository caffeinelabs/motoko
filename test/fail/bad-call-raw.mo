import P "mo:⛔";

actor self {
  let a = P.call_raw(P.principalOfBlob(""), "foo", ""); // reject, send capability required
};

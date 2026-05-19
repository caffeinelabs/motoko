// Companion to gadt-existential-leak-pair: same leak shape but the
// existential lives in a bare tuple alias (`type Pack = type X in
// (X, X)`) rather than a variant arm. Different mechanism — bare
// alias instantiation gives fresh cons per use site, no TagP fresh
// skolem mint — same outcome: tuple sub forces `X <: Nat` which
// fails by Cardelli abstraction.

type Pair<T> = (T, T);
type Pack = type X in (X, X);

let payload = 42;
let p : Pack = (payload, payload);

let _pair : Pair<Nat> = switch p {
   case (different, _) (different, payload)
};

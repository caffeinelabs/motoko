// and-pattern in inference position with *neither* leg annotated —
// hits the dedicated M0261 "cannot infer and-pattern" code. Compare
// `test/run/and-pattern-infer.mo`, which exercises the inference
// path that now succeeds when at least one leg is explicit.
func f(x and y) : Nat = x;
ignore f 3

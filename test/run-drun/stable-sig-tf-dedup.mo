//MOC-FLAG -A=M0194
// Regression: dedup must substitute through actor type-component field
// bodies, else a tf-only cons leaks a dangling name (compiler fails on
// unparseable stable type signatures).
//
// Construction: two modules export structurally-equal `type Inner`. The
// two cons group under [(name, hash body)], dedup picks one as rep, the
// other becomes a loser with no decl emitted. Each [svc{1,2}] has a
// tf-only [type Inner = M{1,2}.Inner] referencing one of them. Without
// substituting through tf-only cons (cs_all in pp_stab_sig), the svc
// holding the loser-reference would print [type Inner = Inner__<loser>]
// against an undeclared name and validate_stab_sig would fail.
import M1 "./stable-sig-tf-dedup/inner";
import M2 "./stable-sig-tf-dedup/inner2";
persistent actor {
  let svc1 : actor {
    type Inner = M1.Inner;
    foo : shared () -> async ();
  } = actor "aaaaa-aa";
  let svc2 : actor {
    type Inner = M2.Inner;
    bar : shared () -> async ();
  } = actor "aaaaa-aa";
};

//MOC-FLAG -A=M0194
// Regression: dedup must substitute through actor type-component field
// bodies, else a tf-only cons leaks a dangling name (compiler fails on
// unparseable stable type signatures).
import M "./stable-sig-tf-dedup/inner";
persistent actor {
  type Inner = { x : Nat; rest : ?Inner };
  let a : Inner = { x = 0; rest = null };
  let svc : actor {
    type Z = M.Inner;
    bar : shared () -> async ();
  } = actor "aaaaa-aa";
};

// Regression: stable-sig dedup hashes every zero-arity Def in cs_all,
// including cons reachable only through actor type-component bodies —
// which need not be stable types. [Typ_hash.typ_hash] is partial: it
// asserts on the polymorphic function alias behind [type P] below.
// Any hash failure must exclude the cons from dedup, not crash the
// compiler.
//
// Also pins that same-named aliases differing only in a type-component
// body stay distinct: [Svc] below groups under one [(name, hash)]
// bucket only if the hash ignores tf bodies, and dedup confirms
// candidates with [Type.eq] rather than trusting hash injectivity.
import M1 "stable-sig-tf-nodedup/svc-nat";
import M2 "stable-sig-tf-nodedup/svc-text";
import W "stable-sig-tf-nodedup/weird";
persistent actor {
  let _w : ?(actor { type P = W.Weird }) = null;
  let _a : ?M1.Svc = null;
  let _b : ?M2.Svc = null;
};

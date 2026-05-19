// Existential leak via tuple reassembly: a `#pack` arm opens an
// abstract witness `X`; the case body returns `(different, payload)`
// where `different : X` and `payload : Nat`. Annotated as
// `Pair<Nat> = (Nat, Nat)`, tuple sub forces `X <: Nat` which fails
// by Cardelli-style abstraction — the fresh destructure skolem is
// `Abs([], Any)` and `Any </: Nat`.
//
// Without per-arm fresh skolems the witness would alias the literal's
// type and the leak would silently type-check. With M11b path B
// (fresh_destructure_skolem per pat.at + σ-substitute in the
// destructure'd arm payload) the leak is statically rejected.

type Pair<T> = (T, T);
type Pack = { #pack : type X in X };

let payload = 42;
let p : Pack = #pack payload;

let _pair : Pair<Nat> = switch p {
   case (#pack different) (different, payload)
};

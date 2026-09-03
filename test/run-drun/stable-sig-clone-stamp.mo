// Regression: dedup clones must not reuse live stamps. [Cons.session]
// restores the per-name stamp counters on exit and compile runs in a
// fresh session scoped to the main unit's filename, so a clone minted
// during rendering could get the same [(name, stamp)] as a checker-era
// cons of the same name declared in the main unit. The two then
// [Cons.eq]-collide: one decl silently vanishes and its stable vars are
// recorded with the other's type. Here [M1.Keeper] (which needs a clone
// because its body refs the deduped [Dup]) collided with the actor's
// local [Keeper], recording [_k : {w : Nat}] and losing [{a : Dup}].
// Clones now mint in a dedicated "stable-sig-dedup" stamp scope.
import M1 "stable-sig-clone-stamp/keeper";
import M2 "stable-sig-clone-stamp/dup";
persistent actor {
  type Keeper = { w : Nat };
  let _own : ?Keeper = null;
  let _k : ?M1.Keeper = null;
  let _d2 : ?M2.Dup = null;
};

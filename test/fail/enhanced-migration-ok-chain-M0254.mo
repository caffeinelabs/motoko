//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG -A=M0194
//MOC-FLAG -W M0254

// Field `x` is not initialized by the chain: it can only come from the
// persisted state of an already-deployed canister. M0254 is an error by
// default; `-W M0254` demotes it to a warning to allow deployment.

actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X}; // extra var, never involved in migrations, inherited from inital
};

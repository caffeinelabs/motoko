//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-noop
//MOC-FLAG -A=M0194,M0244

// The migration chain composes fine but initializes none of the actor's
// stable fields. Since stable fields cannot have initializers under
// --enhanced-migration, a fresh installation would trap at runtime.
// M0254 is an error by default, so this must be rejected statically.

actor {
    var name : Text;
    var balance : Nat;
};

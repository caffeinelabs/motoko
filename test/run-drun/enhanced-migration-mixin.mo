//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-mixin/migrations --generate-view-queries
import Mixin "enhanced-migration-mixin/Mixin";

actor {
    let actorInt : Int;
    ignore actorInt;

    include Mixin();

    let actorText : Text;
    ignore actorText;
};

//SKIP run-ir
//SKIP run-low
//SKIP run

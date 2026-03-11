//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration-imported-class/migrations
import Class "enhanced-migration-imported-class/Class";
actor {
    transient let c = Class.Class();
    let f : {#f};
};

//SKIP run-ir
//SKIP run-low
//SKIP run


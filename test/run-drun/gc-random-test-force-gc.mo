//MOC-FLAG --stable-regions

import GCRandomTest "gc-random-test/gc-random-test";

actor {
    transient let test = GCRandomTest.GCRandomTest();
    
    public shared func run() : async () {
        await test.run(25);
    };
};

//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress run "DIDL\x00\x00"

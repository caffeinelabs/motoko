func foo() : async/*WHY NEEDED?*/ from_candid "service B : { \"getInt\": () -> (int); }" {
    actor { public func getInt() : async Int { 42 } }
}

//SKIP run

func foo() : async/*WHY NEEDED?*/ from_candid "type T = int; service B : { \"getInt\": () -> (T); }" {
    actor { public func getInt() : async Int { 42 } }
}

//SKIP run

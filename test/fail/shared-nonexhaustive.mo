// patterns on shared functions must be exhaustive

shared func wrong1(true : Bool) : async () {};

((shared func wrong2(true : Bool) : async () {}) : shared Bool -> async ());

shared func wrong2(true : Bool) : () {};

((shared func wrong2(true : Bool) : () {}) : shared Bool -> ());

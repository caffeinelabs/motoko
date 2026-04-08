// / # should trap when the variant IS the excluded tag
let v : {#ok : Nat; #err : Text} = #ok 42;
let _ = v / #ok;

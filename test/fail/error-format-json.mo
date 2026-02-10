//MOC-FLAG --error-format json
//MOC-FLAG -W=M0236
//MOC-FLAG --package core $MOTOKO_CORE

import Array "mo:core/Array";

do {
  let true = true;
};

let _ : Nat = "abc";

let ar = [1];
let _ = Array.filter<Nat>(ar, func x = x > 0);
// Expected spans with suggested replacements:
// {"file":"error-format-json.mo","line_start":14,"column_start":9,"line_end":14,"column_end":14,"suggested_replacement":"ar"}
// {"file":"error-format-json.mo","line_start":14,"column_start":27,"line_end":14,"column_end":31,"suggested_replacement":""}
let _ = ar.filter<Nat>(func x = x > 0); // expected no replacements

// TODO: More tests, move tests from MotokoFixer

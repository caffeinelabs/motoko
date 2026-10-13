//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed -A=M0210
import Prim "mo:⛔";

var effects = "";
let result = (with cycles = do { effects #= "note;"; 0 })
  (do { effects #= "callee;"; Prim.fixtureShape })(
    do { effects #= "argument;"; 7 : Nat }
  );
assert effects == "note;callee;argument;";
assert result == "n";

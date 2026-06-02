//MOC-FLAG -W=M0236 --all-libs --package core $MOTOKO_CORE --error-format=json
// Regression: diagnostic columns must count Unicode codepoints, not UTF-8 bytes.
// Without the fix, the second and third lines report end columns 31 and 32
// (byte-based), which over-deletes when tools apply `suggested_replacement`
// spans using codepoint/UTF-16 indexing (e.g. mops --fix, #6132).
import Char "mo:core/Char";
module {
  public func go() {
    ignore Char.toNat32('A');   // ASCII
    ignore Char.toNat32('京');  // 3-byte UTF-8
    ignore Char.toNat32('💩'); // 4-byte UTF-8 (non-BMP)
  };
};

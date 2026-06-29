//MOC-FLAG -W=M0236 --all-libs --package core $MOTOKO_CORE --error-format=json
// Regression: column_start/column_end count Unicode codepoints (matching what
// editors display), and byte_start/byte_end give unambiguous UTF-8 byte offsets
// for machine-applied edits. Pre-fix, column_end was byte-based and tools
// applying `suggested_replacement` spans by codepoint over-deleted on the
// non-ASCII cases.
import Char "mo:core/Char";
module {
  public func go(c : Char) {
    ignore Char.toNat32(c);                       // ASCII baseline
    ignore "京"; ignore Char.toNat32(c);          // 3-byte UTF-8 before span
    ignore "💩"; ignore Char.toNat32(c);          // 4-byte UTF-8 (non-BMP) before span
    ignore "京京"; ignore Char.toNat32(c);        // multibyte text before span
  };
};

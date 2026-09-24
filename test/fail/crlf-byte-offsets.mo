//MOC-FLAG -W=M0236 --error-format=json
// CRLF line endings (kept by .gitattributes): byte_start/byte_end must match the file bytes on every line,
// since `mops check --fix` applies edits by byte offset. Columns still count codepoints.
module M { public func size(self : Nat) : Nat = self };
let m = 1;
ignore M.size(
  m,
);
let s = "éé"; ignore M.size(m);

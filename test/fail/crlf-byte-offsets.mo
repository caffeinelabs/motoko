//MOC-FLAG -W=M0236 --error-format=json
// Regression: this file has CRLF line endings (kept by .gitattributes).
// byte_start/byte_end on lines after a CRLF used to be one byte early,
// so `mops check --fix` garbled the file. Columns still count codepoints.
module M { public func size(self : Nat) : Nat = self };
let m = 1;
ignore M.size(
  m,
);
let s = "éé"; ignore M.size(m);

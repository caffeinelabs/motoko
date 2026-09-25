//MOC-FLAG -W=M0236 --error-format=json
// The code lines below end with a lone CR (kept by .gitattributes), which the lexer treats as a line break.
// M0236 reads its receiver text from the file, so every call below must still get its suggestion.
// The header lines end with LF because run-test reads the test flags line by line.
module M { public func size(self : Nat) : Nat = self };
let m = 1;
ignore M.size(m);
ignore M.size(m);
ignore M.size(
  m,
);

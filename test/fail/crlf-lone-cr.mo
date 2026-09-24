//MOC-FLAG -W=M0236 --error-format=json
// Regression: the code lines below end with a lone CR (kept by .gitattributes),
// which the lexer treats as a line break. The M0236 suggestion reads its
// receiver text from the file and used to split lines on LF only, so it read
// the wrong line and silently dropped every suggestion after the first CR.
module M { public func size(self : Nat) : Nat = self };let m = 1;ignore M.size(m);ignore M.size(m);ignore M.size(  m,);

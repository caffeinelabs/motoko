switch 'a' {
  case 'b' { assert false }
  case 'a' { assert true }
  case _ { assert false }
};

switch (?'\u{28ae4}') {
  case ?'a' { assert false }
  case ?'\u{28ae4}' { assert true }
  case _ { assert false }
}

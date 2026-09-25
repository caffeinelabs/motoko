switch "foo" {
  case "bar" { assert false }
  case "" { assert false }
  case "foo" { assert true }
  case _ { assert false }
};

switch (?"foo") {
  case ?"bar" { assert false }
  case ?"" { assert false }
  case ?"foo" { assert true }
  case _ { assert false }
};

switch "foo
" {
  case "foo
" { assert true }
  case _ { assert false }
};

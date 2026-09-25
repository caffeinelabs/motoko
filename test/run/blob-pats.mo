// Same as text-pats.mo, but uses blobs

switch ("foo" : Blob) {
  case "bar" { assert false }
  case "" { assert false }
  case "foo" { assert true }
  case _ { assert false }
};

switch (?("foo" : Blob)) {
  case ?"bar" { assert false }
  case ?"" { assert false }
  case ?"foo" { assert true }
  case _ { assert false }
}

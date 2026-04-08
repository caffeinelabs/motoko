actor a {
  public func go() : () {
    let _ = async { ():Any };
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"

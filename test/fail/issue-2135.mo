import C = "lib/generic-actor"

actor a {
  public func f () : async ()  { let ci = C.C<Int>(); };
}

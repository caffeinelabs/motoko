//MOC-FLAG -A=M0194
// test handling of type components appearing in types of stable variables
// (used to crash a sanity check in the compiler which could emit but not parse
// type components in signature (.most) files)
actor {

  private type List<A> = ?(A,List<A>);

  private type Bound<A <: {}> = {};

  stable var v : [
    actor {
      type T = Int;
      type U<A> = (A, A);
      type List<A> = List<A>; // references previous List type, not this List component (type components aren't recursive, let alone mutually recursive)
      type Pair<A,B> = (A,B);
      type Bound<A <: {}> = Bound<A>; // ditto
    }
  ] = [];

}

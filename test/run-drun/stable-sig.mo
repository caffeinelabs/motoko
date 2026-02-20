//MOC-FLAG -A=M0194
actor {

  type Nats = ?(Nat,Nats);
  type List<T> = ?(T,List<T>);

  stable let n = 0;
  stable var vn = 0;

  stable let ns = null : Nats ;
  stable let ln : List<Nat> = null : List<Nat>;
  stable let lt = null : List<Text>;
  stable let lnon : List<None> = null : List<None>;
  stable let lany : List<Any> = null : List<Any>;

}

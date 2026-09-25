//MOC-FLAG -A=M0194
actor {

  type Nats = ?(Nat, Nats);
  type List<T> = ?(T, List<T>);

  let n = 0;
  var vn = 0;

  let ns = null : Nats;
  let ln : List<Nat> = null : List<Nat>;
  let lt = null : List<Text>;
  let lnon : List<None> = null : List<None>;
  let lany : List<Any> = null : List<Any>;

}

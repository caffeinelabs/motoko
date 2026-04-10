import C "counter";

module {
  public type AliasedCounter = C.Counter;

  public type Result<T> = { #ok : T; #err : Text };

  public func wrapOk<T>(v : T) : Result<T> = #ok v;

  public func wrapErr<T>(msg : Text) : Result<T> = #err msg;

  public func mapResult<A, B>(r : Result<A>, f : A -> B) : Result<B> {
    switch r {
      case (#ok a) #ok (f a);
      case (#err e) #err e;
    }
  };
}

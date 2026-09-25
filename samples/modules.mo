module Collections = {
  public module List = {
    public type List<T> = ?(T, List<T>);

    public func nil<T>() : List<T> = null;
    public func cons<T>(x : T, l : List<T>) : List<T> = ?(x, l);
  };
};

let List = Collections.List;

type Stack = List.List<Int>;

let empty : Stack = List.nil<Int>();
func push(x : Int, s : Stack) : Stack = List.cons<Int>(x, s);

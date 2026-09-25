import List = "ListLib"; // private, so we don't re-export List

module {

  public type Stack = List.List<Int>;

  public func push(x : Int, s : Stack) : Stack = List.cons<Int>(x, s);

  public func empty() : Stack = List.nil<Int>();

}

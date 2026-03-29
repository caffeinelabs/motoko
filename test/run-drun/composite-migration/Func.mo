module {

  public func then<A, B, C>(self : A -> B, next : B -> C) : A -> C =
    func(a : A) : C { next(self(a)) };

}

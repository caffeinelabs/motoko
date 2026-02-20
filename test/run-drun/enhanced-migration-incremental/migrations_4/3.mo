module {

  public func run(old : { b : Int }) : { b : Bool } {
    {
      b = old.b > 5;
    };
  }

};

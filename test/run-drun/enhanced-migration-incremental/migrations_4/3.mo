module {

  public func migration(old : { b : Int }) : { b : Bool } {
    {
      b = old.b > 5;
    };
  }

};

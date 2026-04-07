module {

  public func migration(_old : {}) : { a : Text } {
    {
      a = "We got here!";
    };
  }

};

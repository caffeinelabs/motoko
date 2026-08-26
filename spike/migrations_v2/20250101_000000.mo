module {
  public func migration(_old : {}) : { greeting : Text } {
    { greeting = "hello" }
  }
}

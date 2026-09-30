import _Show "Show";

module {
  public let Show = _Show;
  public module Inner {
    public let Show = _Show;
  };
}

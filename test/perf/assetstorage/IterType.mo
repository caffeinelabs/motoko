/// The Iterator type

// Just here to break cyclic module definitions

module {
  public type Iter<T> = { next : () -> ?T };
}

/// A module for testing field doc comments.
module {
  /// A record type with documented fields.
  public type Point = {
    /// The X coordinate.
    x : Float;
    /// The Y coordinate.
    y : Float;
    name : Text;
  };

  /// A variant type with documented tags.
  public type Shape = {
    /// A circle with a given radius.
    #circle : Float;
    /// A rectangle with width and height.
    #rect : (Float, Float);
    #dot;
  };
}

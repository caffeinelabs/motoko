module {
    // Consume consumed, produce bad, dropped and lossy.
    public func migration(old : { consumed : Nat })
      : { bad : Int; dropped : Nat; lossy : { a : Nat; b : Nat } } {
        { bad = old.consumed; dropped = 0; lossy = { a = 0; b = 0 } };
    };
};

module {
    // Transform a, introduce b.
    public func migration(old : { a : Nat }) : { a : Nat; b : Nat } {
        { a = old.a; b = 1 }
    };
};

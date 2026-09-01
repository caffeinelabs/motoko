module {
    // Rename b back to a.
    public func migration(old : { b : Nat }) : { a : Nat } { { a = old.b } };
};

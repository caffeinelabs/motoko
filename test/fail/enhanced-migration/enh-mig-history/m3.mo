module {
    // Rename a to b.
    public func migration(old : { a : Nat }) : { b : Nat } { { b = old.a } };
};

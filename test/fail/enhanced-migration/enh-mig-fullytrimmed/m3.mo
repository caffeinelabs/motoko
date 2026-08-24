module {
    // Convert bad and drop dropped, both deployed by the trimmed-away history.
    public func migration(old : { bad : Int; dropped : Nat }) : { bad : Nat } {
        { bad = old.dropped };
    };
};

module {
    // Keep a; the baseline records this migration as {} -> {a : Nat}.
    public func migration(old : { a : Nat }) : { a : Nat } { old };
};

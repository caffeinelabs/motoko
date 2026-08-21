module {
    // New since the deploy: requires a field the deployed state lacks and
    // misdeclares the type of one it has.
    public func migration(old : { consumed : Bool; nonexistent : Text })
      : { done : Nat } {
        { done = if (old.consumed) 1 else 0 };
    };
};

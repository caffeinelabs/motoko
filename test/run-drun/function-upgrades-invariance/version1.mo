// Compatible upgrade: adds a stable variable. Its label sorts before
// `callbacks`, so the added record types take lower type-table indices and push
// the callback parameter record to a higher index in the new type table than
// the whole old type table has entries. The transposed cache access then runs
// past the end of the bit table.
//MOC-FLAG --sanity-checks
actor {
  stable var anchor : { p : { q : Nat } } = { p = { q = 0 } };
  stable var callbacks : [var (shared { x : Nat } -> ())] = [var];
};

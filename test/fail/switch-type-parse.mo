// Slice-4: `switch type T { … }` parses to a plain SwitchE.  Until
// slice 5/6 wire the macro-expansion + value-side of `<T with type>`,
// T is an unbound variable at this position, so M0057 fires.  This
// test migrates to test/run/ once those slices land.
func f<T with type>(arg : T) : ?Int {
  switch type T {
    case Int (?42);
    case Any (null);
  }
};

let emptyIter = { next = func () : Null { return null } }; // <: { next: () -> ? None };
for i in emptyIter {
  // should be accepted
};

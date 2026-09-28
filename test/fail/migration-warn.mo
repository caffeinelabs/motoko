// Warn about fields that are consumed, not produced and re-declared.
// These fields will be re-initialized, not retained.
// This may be intentional or a sign of data loss, hence just a warning.
(with migration =
  func({ data : Nat }) : {} {
    ignore data;
    {}
  })
actor A {

  var data = 0;

  ignore data;

};

// fix 1: remove from domain
(with migration =
  func({}) : {} {
    {}
  })
actor B {

  var data = 0;

  ignore data;

};

// fix 2: add to range
(with migration =
  func({ data : Nat }) : { data : Nat } {
    { data }
  })
actor C {

  var data = 0;

  ignore data;

};

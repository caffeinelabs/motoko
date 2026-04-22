// simplified version of data-view.mo that doesn't require core.
//MOC-FLAG --generate-view-queries
persistent actor Self {

  type T0 = {#T0};
  module View0 {
    public func view<V>(self : T0) :
      () -> T0 =
      func () = self
  };

  let v0 : T0 = #T0;

  type T1 = {#T1};
  module View1 {
    public func view<V>(self : T1) :
      (t1:T1) -> T1 =
      func (t1) = self
  };

  let v1 : T1 = #T1;


  type T2 = {#T2};
  module View2 {
    public func view<V>(self : T2) :
      (t1:T2, t2:T2) -> T2 =
      func (t1, t2) = self
  };

  let v2 : T2 = #T2;

  type T3 = {#T3};
  module View3 {
    public func view<V>(self : T3) :
      (t1:T2, t2:T2, t3: T3) -> T3 =
      func (t1, t2, t3) = self
  };

  let v3 : T3 = #T3;

}

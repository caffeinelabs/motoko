module UsedModule {
  public module SubModule {
    public type T1 = Nat;
  };
};

module UnusedModule {
  module UnusedSubModule {};
};

type T2 = UsedModule.SubModule.T1;

actor a {
    public func getNat() : async Nat { 0 };
    flexible var x : [async Nat] = [];
    public func foo1() : () { x := [a.getNat()] };
    public func foo2() : async Nat {
                10 +
         (await x[0])
    };
};

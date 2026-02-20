//MOC-FLAG -A=M0194
actor a {
    public func go() : async () {
        let r = async ({ b = 42 });
        let s : { b : Nat; c : Char } = { (await r) with c = 'C' };
        let t : { b : Nat; cs : Text } = { (await async s) with cs = await async "Hi"  };
        return;
    }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"

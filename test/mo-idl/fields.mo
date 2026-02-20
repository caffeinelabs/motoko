//MOC-FLAG -A=M0194
actor Server {
    public type Info = {
        name : Text;
        _1_ : Info;
        _345_ : Text;
        _34_5_: Bool;
        _name_: Int;
        name__ : Nat;
    };
    public type Tup = (Text, Info, Text, Nat);
    public func f_(user : Tup) : async () {
    };
    public func f__(user : ?Tup) : () { };
}


//MOC-FLAG -A=M0194
actor Server {
    public type Info = {
        name : Text;
        _1_ : Info;
        _345_ : Text;
        name_: Int;
    };
    public type Tup = (Text, Info, Text, Nat);
    public query func f_(user : Tup) : async () {
    };
    public func f__(user : ?Tup) : () { };
}


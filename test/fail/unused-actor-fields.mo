//MOC-FLAG -A=M0240
actor {
    let _noUnusedWarning = 1;

    class TestClass(classParameter1 : Nat, classParameter2 : Int) {
        let field1 = 1;
        var field2 = 2;
        var field3 = field2;

        func unusedMethod(unusedParameter1 : Any) {
            let local1 = 0;
        };
    };

    object TestObject {
        let objectField1 = 1;
        var objectField2 = 2;
        let objectField3 = objectField1;
        var objectField4 = objectField2;
    };

    var variable1 = 0;
    var variable2 = "TEST";

    func testUnusedFunction(parameter1 : Nat, parameter2 : Int) {
        var variable2 = 2;
        var variable3 = 3;
        let variable4 = 4;
        var variable5 = 5;
        if (variable1 == 0 and variable3 == 3) {
            let localFunction = func() {
                let variable2 = parameter1;
                variable5 *= variable2;
            };
        };
    };

    public shared ({ caller }) func publicActorFunction() : async () {};
    public func otherPublicActorFunction(unusedParameter2 : Text) : () {};
};

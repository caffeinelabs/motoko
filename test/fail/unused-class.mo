class UnusedTestClass() {
  let field = 0;
};

class UsedTestClass(classParameter1 : Nat, classParameter2 : Int) {
  public let field1 = 1;
  var field2 = 2;
  let field3 = field1;
  var field4 = field2;

  func unusedMethod(unusedParameter : Any) {
    let local1 = 0;
    let local2 = local1;
  };
};

let test = UsedTestClass(0, 0);
let result = test.field1;

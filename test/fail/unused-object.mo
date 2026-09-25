object UnusedTestObject {
  let field = 0;
};

object UsedTestObject {
  public let field1 = 1;
  var field2 = 2;
  let field3 = field1;
  var field4 = field2;

  func unusedMethod(unusedParameter : Any) {
    let local1 = 0;
    let local2 = local1;
  };
};

let result = UsedTestObject.field1;

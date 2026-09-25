let variable1 = 1;
var _variable2 = 2;

func _testUnusedFunction(parameter1 : Nat, _parameter2 : Int) {
  var _variable2 = 2;
  let variable3 = 3;
  var _variable4 = 4;
  var variable5 = 5;
  if variable1 == 0 and variable3 == 3 {
    let _localFunction = func() {
      let variable2 = parameter1;
      let parameter2 = 2;
      variable5 *= variable2 + parameter2;
    };
  };
};

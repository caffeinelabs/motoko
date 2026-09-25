let variable1 = 1;
var variable2 = 2;

func testUnusedFunction(parameter1 : Nat, parameter2 : Int) {
  var variable2 = 2;
  let variable3 = 3;
  var variable4 = 4;
  var variable5 = 5;
  if variable1 == 0 and variable3 == 3 {
    let localFunction = func() {
      let variable2 = parameter1;
      let parameter2 = 2;
      variable5 *= variable2 + parameter2;
    };
  };
};

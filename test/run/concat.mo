//MOC-FLAG -A=M0194
func testText(a : Text, b : Text) {
  let cat1 = a # b;
  let cat2 = (a # b) : Text;
  var cat3 = a;
  cat3 #= b;
}

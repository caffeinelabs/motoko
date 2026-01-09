func main1() {
  var n = 0;
  var sum = 0;
  for (i in [1, 2, 3, 4, 5, 6, 7].values()) {
    n += 1;
    if (n == 2) continue;
    if (n == 5) break;
    sum += n;
  };
  assert (sum == 1 + 3 + 4);
};
func main2() {
  var n = 0;
  var sum = 0;
  while (true) {
    n += 1;
    if (n == 2) continue;
    if (n == 5) break;
    sum += n;
  };
  assert (sum == 1 + 3 + 4);
};
func main3() {
  var n = 0;
  var sum = 0;
  loop {
    n += 1;
    if (n == 2) continue;
    if (n == 5) break;
    sum += n;
  } while (true);
  assert (sum == 1 + 3 + 4);
};
func main4() {
  var n = 0;
  var sum = 0;
  loop {
    n += 1;
    if (n == 2) continue;
    if (n == 5) {
      assert (sum == 1 + 3 + 4);
      return;
    };
    sum += n;
  }
};
func main5() {
  var n = 0;
  var sum = 0;
  label l loop {
    n += 1;
    if (n == 2) continue;
    if (n == 5) break l; // mixing unlabeled and labeled break/continue
    sum += n;
  };
  assert (sum == 1 + 3 + 4);
};
func main6() {
  var n = 0;
  var sum = 0;
  loop {
    n += 1;
    if (n == 2) continue;
    if (n == 5) break;
    sum += n;
  };
  assert (sum == 1 + 3 + 4);
};
main1();
main2();
main3();
main4();
main5();
main6();

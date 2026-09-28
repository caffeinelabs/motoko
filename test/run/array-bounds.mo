let a = [0, 1, 2, 3, 4];

var n = 0;
while n <= a.size() {
  assert(n == a[n]);
  n := n + 1;
}

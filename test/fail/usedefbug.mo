//MOC-FLAG -A=M0194
class Bar () {

  private class Foo(f1:Int -> Int, f2:Int -> Int) {
    let bomb = f1(666) + f2(666);
  };

  let Bar = Foo(g, g);

  private func g(n:Int) : Int = n + 1;

}

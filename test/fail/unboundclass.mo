class Foo(f1:Int -> Int, f2:Int -> Int) {};

class Bar () {

  let Bar = Foo(g, g);

  private func g(n:Int) : Int = n + 1;

}

class Foo(f1:Int -> Int, f2:Int -> Int) {};

class Bar () {

  private func g(n:Int) : Int = n + 1;

  let Bar = Foo(g, g);

};

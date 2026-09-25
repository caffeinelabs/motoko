class Foo(f1 : Int -> Int, f2 : Int -> Int) {};

class Bar() {
  func g(n:Int) : Int = n + 1;

  let foo = Foo(g, g);
};

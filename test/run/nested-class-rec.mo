//MOC-FLAG -A=M0194

class Foo(f1:Int -> Int, f2:Int -> Int) { };

class Bar () {
  func g(n:Int) : Int = n + 1;

  public let Bar = Foo(g, g) : Foo; // annotation needed to typecheck constructor call
};

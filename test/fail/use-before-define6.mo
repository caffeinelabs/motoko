//MOC-FLAG -A=M0194
class Bar () {
  class Foo(f1 : Int -> Int, f2 : Int -> Int) {
    let bomb = f1(666) + f2(666);
  };

  let _ = Foo(g, g);

  func g(n : Int) : Int = n + 1;
};

let _ = Bar();

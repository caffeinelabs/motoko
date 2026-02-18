//MOC-FLAG -A=M0194

func f(x : C) = ();

class C() = this {
  public func apply() : () = f(this);
};

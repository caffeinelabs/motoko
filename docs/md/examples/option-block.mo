type Exp = {
  #Lit : Nat;
  #Div : (Exp, Exp);
  #IfZero : (Exp, Exp, Exp);
};

func eval(e : Exp) : ? Nat {
  do ? {
    switch e {
      case (#Lit n) { n };
      case (#Div (e1, e2)) {
        let v1 = eval e1 !;
        let v2 = eval e2 !;
        if (v2 == 0)
          null !
        else v1 / v2
      };
      case (#IfZero (e1, e2, e3)) {
        if (eval e1 ! == 0)
          eval e2 !
        else
          eval e3 !
      };
    };
  };
}

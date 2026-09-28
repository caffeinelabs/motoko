type Exp = {
  #Lit : Nat;
  #Div : (Exp, Exp);
  #IfZero: (Exp, Exp, Exp)
};

func eval(e: Exp) : ? Nat {
  do ? {
    switch e {
      case #Lit(n) { n }
      case #Div(e1, e2) {
        let v1 = eval e1 !;
        let v2 = eval e2 !;
        if v2 == 0
          { null ! } // silly bang
        else { (v1 / v2) };
      }
      case #IfZero(e1, e2, e3) {
        if eval(e1) ! == 0
          { eval e2 ! }  // tail recursive?
        else
          { eval e3 ! } // tail recursive?
      }
    }
  }
}

/* Similar, for Results
func eval( e: Exp) : Result<Nat,Text> {
  // label "!" { #Ok (...) }
  do #Ok {
    // e ! =def= case e of #Ok v -> v | #Err e -> break "!" (#Err e);
    switch e {
       case (#Lit(n)) { n };
       case (#Div (e1, e2)) {
         let v1 = eval e1 !;
         let v2 = eval e2 !;
         if (v2 == 0)
           (#Err "DIV/0") !
         else (v1 / v2);
       };
       case (#IfZero (e1, e2, e3)) {
         if (eval e1 ! == 0)
           eval e2 !
         else
           eval e3 !
       }
    }
  }
}
*/

type Exp = { #Lit : Nat;
             #Div : (Exp, Exp);
             #IfZero: (Exp, Exp, Exp)
           };

func eval( e: Exp) : ? Nat {
  do ? {
    switch e {
       case (#Lit(n)) { n };
       case (#Div (e1, e2)) {
         let v1 = eval e1 !;
         let v2 = eval e2 !;
         if (v2 == 0)
           null ! // silly bang
         else (v1 / v2);
       };
       case (#IfZero (e1, e2, e3)) {
         if (eval e1 ! == 0)
           eval e2 !  // tail recursive?
         else
           eval e3 ! // tail recursive?
       }
    }
  }
};

func evalRes(e: Exp) : {#ok : Nat; #err : Text} {
  do #ok {
    switch e {
       case (#Lit(n)) { n };
       case (#Div (e1, e2)) {
         let v1 = evalRes e1 !;
         let v2 = evalRes e2 !;
         if (v2 == 0)
           (#err "DIV/0") !
         else (v1 / v2);
       };
       case (#IfZero (e1, e2, e3)) {
         if (evalRes e1 ! == 0)
           evalRes e2 !
         else
           evalRes e3 !
       }
    }
  }
};


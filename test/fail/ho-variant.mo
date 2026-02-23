//MOC-FLAG -A=M0194

type Term = {
 #id : Text;
 #lit : Int;
 #lam : (Text, Term);
 #app : (Term, Term);
 #pair : (Term, Term);
 #fst : Term;
};

type Val = {
 #int : Int;
 #fun : Val -> Val; // no parens required
 #pair : (Val,Val);
 #wrong
};

let p = (1,2);
let a = [0];
let ap = [p];
let x0 = #int 1;
let x1 = #int (p.1);
let x2 = #int (ap[0].1);

let y : Val = #fun (func v = v);
let z = #wrong;

func eval (env : Text -> Val, e:Term) : Val {
 switch e {
   case (#id x) env(x);
   case (#lit i) #int i;
   case (#lam (x,e)) #fun (func v { eval(func y { if (y == x)  v else env y }, e)});
   case (#app (e1, e2)) {
     switch (eval(env,e1), eval (env,e2)) {
       case (#fun f,v) f(v);
       case _ #wrong;
     };
   };
   case (#pair (e1, e2)) #pair (eval(env, e1), eval(env, e2));
   case (#fst e) {
     switch (eval(env, e)) {
       case (#pair (v1,_)) v1;
       case _ #wrong;
     };
   }
 };
};

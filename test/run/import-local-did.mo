//MOC-FLAG -A=M0194
import S "import-local-did/service.did";
import { type Self; type UserId; type T } "import-local-did/service.did";

func same<A>(_ : A, _ : A) {};

let _ : S.Self = (actor "aaaaa-aa" : S.Self);
same<S.UserId>(0, 0);
same<S.AccountBalance>(0, 0);
same<S.T>("", "");
same<UserId>(0, 0);
same<T>("", "");
same<Self>(actor "aaaaa-aa" : Self, actor "aaaaa-aa" : S.Self);

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP comp

//MOC-FLAG --actor-id-alias svc aaaaa-aa import-local-did-dual/aaaaa-aa.did
//MOC-FLAG -A=M0194
import Types "import-local-did-dual/aaaaa-aa.did";
import Svc "canister:svc";

func same<A>(_ : A, _ : A) {};

same<Types.Self>(Svc, Svc);
same<Types.UserId>(0, 0);
ignore Svc.get;

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP comp

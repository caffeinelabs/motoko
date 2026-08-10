//MOC-FLAG -A=M0194
import S "idl:import-local-did/service.did";
import { type Self; type UserId; type T } "idl:import-local-did/service.did";

func same<A>(_ : A, _ : A) {};

func use(x : S.UserId) : S.UserId { x };
assert (use(0) == 0);

same<S.AccountBalance>(0, 0);
same<S.T>("", "");
same<UserId>(0, 0);
same<T>("", "");
func asLedger(_ : Self) {};
func asLedger2(_ : S.Self) {};

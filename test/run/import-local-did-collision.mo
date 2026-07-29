//MOC-FLAG -A=M0194
import S "import-local-did-collision/service.did";

func same<A>(_ : A, _ : A) {};

// Collision: user_id stays snake_case; UserId stays as-is.
same<S.user_id>(0, 0);
same<S.UserId>("", "");

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP comp

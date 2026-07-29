//MOC-FLAG -A=M0194
import S "import-local-did-collision/service.did";

func same<A>(_ : A, _ : A) {};

// user_id vs UserId: keep originals
same<S.user_id>(0, 0);
same<S.UserId>("", "");

// http_request vs HTTP_request both want HttpRequest → keep originals
same<S.http_request>(0, 0);
same<S.HTTP_request>("", "");

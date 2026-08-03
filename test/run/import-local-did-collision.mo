//MOC-FLAG -A=M0194
import S "idl:import-local-did-collision/service.did";

func same<A>(_ : A, _ : A) {};

// user_id vs UserId: UserId is taken, user_id keeps its name
same<S.user_id>(0, 0);
same<S.UserId>("", "");

// HTTP_request keeps its name (only lowercase-leading ids are renamed), so http_request gets HttpRequest
same<S.HttpRequest>(0, 0);
same<S.HTTP_request>("", "");

// foo_bar vs fooBar both want FooBar: keep originals
same<S.foo_bar>(0, 0);
same<S.fooBar>("", "");

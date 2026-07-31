//MOC-FLAG -A=M0194
import S "idl:file:import-local-did-collision/service.did";

func same<A>(_ : A, _ : A) {};

// user_id vs UserId: UserId is taken, user_id keeps its name
same<S.user_id>(0, 0);
same<S.UserId>("", "");

// HTTP_request is not renamed (acronym-leading); http_request still PascalCases
same<S.HttpRequest>(0, 0);
same<S.HTTP_request>("", "");

// foo_bar vs fooBar both want FooBar: keep originals
same<S.foo_bar>(0, 0);
same<S.fooBar>("", "");

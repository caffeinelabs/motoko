// Phantom-state machine — the OAuth pattern. Each tag is a distinct
// singleton-variant type so the refinement can actually tell them
// apart at the type level (the obvious `type Idle = ()` aliasing
// trick collapses everything to `()` and refinement can't
// discriminate).

type Idle    = { #idle };
type Pending = { #pending };
type Authed  = { #authed };

type State<S> = {
  #idle    : type S = Idle    in ();
  #pending : type S = Pending in Text;
  #authed  : type S = Authed  in Text;
};

func start ()                  : State<Idle>    = #idle ();
func initiate (verifier : Text) : State<Pending> = #pending verifier;
func complete (token : Text)    : State<Authed>  = #authed token;

func getVerifier (s : State<Pending>) : Text =
  switch s { case (#pending v) v };

func getToken (s : State<Authed>) : Text =
  switch s { case (#authed t) t };

let s0 = start();
let s1 = initiate "verifier_abc";
let s2 = complete "token_xyz";

assert getVerifier s1 == "verifier_abc";
assert getToken    s2 == "token_xyz";
ignore s0;

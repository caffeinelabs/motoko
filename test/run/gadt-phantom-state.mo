// Full OAuth state machine with refresh-token dance.
//
//   [*] -> Idle
//   Idle    -> Pending  via initiate(verifier)
//   Pending -> Authed   via complete(token, refresh, expiry)
//   Authed  -> Expired  via expire()                    (401 / clock-driven)
//   Expired -> Authed   via refreshToken(new, expiry)   (refresh-token call)
//   Authed  -> Revoked  via revokeAuthed()              (logout / refresh fail)
//   Expired -> Revoked  via revokeExpired()             (refresh fail)
//   Revoked -> Idle     via reset()
//
// Each state index is a distinct singleton variant so refinement can
// discriminate at the type level. State payloads differ per state:
// Pending carries the PKCE verifier, Authed the full session,
// Expired only the refresh token, Idle/Revoked nothing.

type IdleT    = { #idle };
type PendingT = { #pending };
type AuthedT  = { #authed };
type ExpiredT = { #expired };
type RevokedT = { #revoked };

type Session = { token : Text; refresh : Text; expiry : Int };
type Stale   = { refresh : Text };

type State<S> = {
  #idle    : type S = IdleT    in ();
  #pending : type S = PendingT in Text;       // PKCE verifier
  #authed  : type S = AuthedT  in Session;
  #expired : type S = ExpiredT in Stale;
  #revoked : type S = RevokedT in ();
};

func start () : State<IdleT> = #idle ();

func initiate (verifier : Text) : State<PendingT> = #pending verifier;

func complete (token : Text, refresh : Text, expiry : Int) : State<AuthedT> =
  #authed { token; refresh; expiry };

// 401 / clock-driven: keep the refresh token, drop the access token.
func expire (s : State<AuthedT>) : State<ExpiredT> =
  switch s {
    case (#authed sess) #expired { refresh = sess.refresh };
  };

// Refresh-token exchange: trade Stale for a fresh Session.
func refreshToken (s : State<ExpiredT>, newToken : Text, newExpiry : Int)
  : State<AuthedT> =
  switch s {
    case (#expired { refresh }) #authed { token = newToken; refresh; expiry = newExpiry };
  };

// Two revoke entry points — without unification, a single function can't
// accept both Authed and Expired in one signature. The types do enforce
// that you've reached a state where revocation is meaningful (no
// revoking an Idle session).
func revokeAuthed  (s : State<AuthedT>)  : State<RevokedT> { ignore s; #revoked () };
func revokeExpired (s : State<ExpiredT>) : State<RevokedT> { ignore s; #revoked () };

// Start over.
func reset (s : State<RevokedT>) : State<IdleT> { ignore s; #idle () };

// Accessors that only work in the right state — calling
// getToken on a State<ExpiredT> is a compile-time error.
func getToken (s : State<AuthedT>) : Text =
  switch s { case (#authed sess) sess.token };

func getRefreshFromAuthed (s : State<AuthedT>) : Text =
  switch s { case (#authed sess) sess.refresh };

func getRefreshFromExpired (s : State<ExpiredT>) : Text =
  switch s { case (#expired stale) stale.refresh };

// Drive the full flow.
let s0 : State<IdleT>    = start ();
let s1 : State<PendingT> = initiate "verifier_abc";
let s2 : State<AuthedT>  = complete ("tok_v1", "rfr_v1", 1000);
assert getToken s2 == "tok_v1";

let s3 : State<ExpiredT> = expire s2;
assert getRefreshFromExpired s3 == "rfr_v1";

let s4 : State<AuthedT>  = refreshToken (s3, "tok_v2", 2000);
assert getToken s4 == "tok_v2";
assert getRefreshFromAuthed s4 == "rfr_v1";  // refresh token preserved across refresh

let s5 : State<RevokedT> = revokeAuthed s4;
let s6 : State<IdleT>    = reset s5;

// Alternate path: refresh-token call fails → revoke from Expired.
let _alt : State<IdleT> = reset (revokeExpired (expire (complete ("t", "r", 0))));

ignore s0;
ignore s1;
ignore s6;

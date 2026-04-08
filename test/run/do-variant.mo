import Prim "mo:⛔";

type Result<T, E> = {#ok : T; #err : E};

func ok<T, E>(v : T) : Result<T, E> = #ok v;
func err<T, E>(v : E) : Result<T, E> = #err v;

// Basic do #ok: success path
let r1 : Result<Nat, Text> = do #ok {
    let a = ok<Nat, Text>(1);
    let b = ok<Nat, Text>(2);
    a! + b!;
};
Prim.debugPrint(debug_show(r1));
assert (r1 == #ok 3);

// do #ok: failure path (propagates #err)
let r2 : Result<Nat, Text> = do #ok {
    let a = ok<Nat, Text>(1);
    let b = err<Nat, Text>("bad");
    a! + b!;
};
Prim.debugPrint(debug_show(r2));
assert (r2 == #err "bad");

// do #ok: multiple error types
type MultiResult = {#ok : Nat; #err : Text; #timeout};

func mkOk(v : Nat) : MultiResult = #ok v;
func _mkErr(v : Text) : MultiResult = #err v;
func mkTimeout() : MultiResult = #timeout;

let r3 : MultiResult = do #ok {
    let a = mkOk(10);
    let b = mkOk(20);
    a! + b!;
};
Prim.debugPrint(debug_show(r3));
assert (r3 == #ok 30);

let r4 : MultiResult = do #ok {
    let a = mkOk(10);
    let b = mkTimeout();
    a! + b!;
};
Prim.debugPrint(debug_show(r4));
assert (r4 == #timeout);

// / # operator: variant narrowing
let v1 : {#ok : Nat; #err : Text} = #err "hello";
let v2 : {#err : Text} = v1 / #ok;
Prim.debugPrint(debug_show(v2));
assert (v2 == #err "hello");

// do #ok with a single-option variant (always succeeds, no propagated tags)
let r7 : {#ok : Nat} = do #ok {
    let a : {#ok : Nat} = #ok 99;
    a!;
};
Prim.debugPrint(debug_show(r7));
assert (r7 == #ok 99);

// do #ok with 4 error tags
type Big = {#ok : Nat; #err : Text; #timeout; #denied : Int; #retry : Nat};

func bigOk(v : Nat) : Big = #ok v;
func bigTimeout() : Big = #timeout;
func bigDenied(v : Int) : Big = #denied v;

let r8 : Big = do #ok {
    let a = bigOk(1);
    let b = bigOk(2);
    a! + b!;
};
Prim.debugPrint(debug_show(r8));
assert (r8 == #ok 3);

let r9 : Big = do #ok {
    let a = bigOk(1);
    let b = bigTimeout();
    a! + b!;
};
Prim.debugPrint(debug_show(r9));
assert (r9 == #timeout);

let r10 : Big = do #ok {
    let a = bigOk(1);
    let b = bigDenied(-42);
    a! + b!;
};
Prim.debugPrint(debug_show(r10));
assert (r10 == #denied(-42));

// / # narrowing on 3-option variant
let v3 : {#ok : Nat; #err : Text; #timeout} = #timeout;
let v4 : {#err : Text; #timeout} = v3 / #ok;
Prim.debugPrint(debug_show(v4));
assert (v4 == #timeout);

// / # narrowing twice, removing two tags
let v5 : {#err : Text; #timeout} = #err "oops";
let v6 : {#err : Text} = v5 / #timeout;
Prim.debugPrint(debug_show(v6));
assert (v6 == #err "oops");

// / # narrowing on 5-option variant down to 4
let v7 : Big = #retry 3;
let v8 : {#err : Text; #timeout; #denied : Int; #retry : Nat} = v7 / #ok;
Prim.debugPrint(debug_show(v8));
assert (v8 == #retry 3);

// / # narrowing a 2-option variant to 1
let v9 : {#ok : Nat; #err : Text} = #err "solo";
let v10 : {#err : Text} = v9 / #ok;
Prim.debugPrint(debug_show(v10));
assert (v10 == #err "solo");

// Nesting: do #ok inside do ?
let r5 : ?(Result<Nat, Text>) = do ? {
    let r : Result<Nat, Text> = do #ok {
        let a = ok<Nat, Text>(5);
        a!;
    };
    r;
};
Prim.debugPrint(debug_show(r5));
assert (r5 == ?(#ok 5));

// Nesting: do ? inside do #ok
// Inside do #ok, ! is for variants, so use switch for options
let r6 : Result<Nat, Text> = do #ok {
    let r = ok<Nat, Text>(42);
    r!;
};
Prim.debugPrint(debug_show(r6));
assert (r6 == #ok 42);

# core/Runtime
Runtime utilities.
These functions were originally part of the `Debug` module.

```motoko name=import
import Runtime "mo:core/Runtime";
```

## Function `trap`
``` motoko no-repl
func trap(errorMessage : Text) : None
```

`trap(t)` traps execution with a user-provided diagnostic message.

The caller of a future whose execution called `trap(t)` will
observe the trap as an `Error` value, thrown at `await`, with code
`#canister_error` and message `m`. Here `m` is a more descriptive `Text`
message derived from the provided `t`. See example for more details.

NOTE: Other execution environments that cannot handle traps may only
propagate the trap and terminate execution, with or without some
descriptive message.

```motoko include=import no-validate
Runtime.trap("An error occurred!");
```

## Function `unreachable`
``` motoko no-repl
func unreachable() : None
```

`unreachable()` traps execution when code that should be unreachable is reached.

This function is useful for marking code paths that should never be executed,
such as after exhaustive pattern matches or unreachable control flow branches.
If execution reaches this function, it indicates a programming error.

```motoko include=import no-validate
let number = switch (?5) {
  case (?n) n;
  case null Runtime.unreachable();
};
assert number == 5;
```

## Function `envVarNames`
``` motoko no-repl
func envVarNames() : [Text]
```

Returns the names of all canister environment variables.

Example:
```motoko include=import no-validate
let names = Runtime.envVarNames();
```

## Function `envVar`
``` motoko no-repl
func envVar(name : Text) : ?Text
```

Returns an optional value of the canister environment variable with the given name.

Example:
```motoko include=import no-validate
let value = Runtime.envVar("MY_ENV_VAR");
let result = switch (value) {
  case (?v) v;
  case null Runtime.trap("Unknown environment variable");
};
```

# core/CallerAttributes
Allows accessing the Internet Computer's caller attributes.
TODO: link to official documentation, once it's available.

```motoko name=import
import CallerAttributes "mo:core/CallerAttributes";
```

## Function `getAttributes`
``` motoko no-repl
func getAttributes() : ?Blob
```

Returns the attribute data attached to the current call, but only
when the signer is listed in the `trusted_attribute_signers`
canister environment variable.

Returns `null` if the current call carries no caller attributes.
Traps if the signer isn't trusted.

`trusted_attribute_signers` is expected to be a comma-separated list
of principal texts, for example:
`"aaaaa-aa,un4fu-tqaaa-aaaab-qadjq-cai"`.

Example:
```motoko include=import no-validate
persistent actor {
  public shared func handle() : async () {
    switch (CallerAttributes.getAttributes()) {
      case (?data) { /* attributes came from a trusted signer */ };
      case null { /* no attributes, or signer is not trusted */ };
    };
  };
}
```

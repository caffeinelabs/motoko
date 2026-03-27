# core/Base64
Module for Base64 encoding of byte sequences.

Base64 encoding converts binary data to an ASCII string using 64 printable
characters, as specified in [RFC 4648](https://www.rfc-editor.org/rfc/rfc4648).
It is widely used for HTTP Basic Authentication, encoding binary data in
JSON payloads, and data URIs.

This module uses the standard Base64 alphabet (`A–Z`, `a–z`, `0–9`, `+`, `/`)
and pads output to a multiple of 4 characters using `=`.

Authored by Claude Sonnet (claude-sonnet-4-6) for use in generated
Motoko API clients.

Import from the core package to use this module.
```motoko name=import
import Base64 "mo:core/Base64";
```

## Function `encode`
``` motoko no-repl
func encode(data : Blob) : Text
```

Encodes a `Blob` as a Base64 `Text` string (RFC 4648 §4).

Output length is always a multiple of 4, padded with `=` as needed.
An empty `Blob` encodes to an empty `Text`.

Example:
```motoko include=import
assert Base64.encode("" : Blob) == "";
assert Base64.encode("f" : Blob) == "Zg==";
assert Base64.encode("fo" : Blob) == "Zm8=";
assert Base64.encode("foo" : Blob) == "Zm9v";
assert Base64.encode("foobar" : Blob) == "Zm9vYmFy";
```

Typical use — embedding text in a data URI:
```motoko include=import
let payload = "Hello" : Blob;
let uri = "data:text/plain;base64," # Base64.encode(payload);
assert uri == "data:text/plain;base64,SGVsbG8=";
```

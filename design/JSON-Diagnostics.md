JSON Diagnostic Output
======================

When `--error-format json` is passed to `moc`, diagnostics are emitted as JSON to stdout.
Each diagnostic is printed as a single line of JSON (one JSON object per line, also known as [JSON Lines](https://jsonlines.org/)).

This format is intended for machine consumption by IDEs, build tools, and
other tooling that needs structured access to compiler diagnostics.

Inspired by [rustc's JSON output](https://doc.rust-lang.org/beta/rustc/json.html),
but simplified to match Motoko's diagnostic model.


Format
------

Each line on stdout is a JSON object with the following structure:

```javascript
{
    /* The primary diagnostic message.
       May contain newlines for multi-line messages
       (e.g. type mismatches showing expected vs. actual types).
    */
    "message": "literal of type\n  Text\ndoes not have expected type\n  Nat",

    /* The Motoko error/warning code, e.g. "M0050", "M0145".
       May be an empty string for info-level diagnostics without a code.
    */
    "code": "M0050",

    /* The severity of the diagnostic.
       Values may be:
       - "error": A fatal error that prevents compilation.
       - "warning": A possible concern that does not block compilation.
       - "info": Informational message.
    */
    "level": "error",

    /* An array of source locations associated with the diagnostic.
       Currently each diagnostic produces exactly one span.
       The array format is used for forward-compatibility with potential multi-span diagnostics in the future.
    */
    "spans": [
        {
            /* File path as passed to the compiler. May be relative or absolute. */
            "file": "myfile.mo",
            /* First line of the span (1-based, inclusive). */
            "line_start": 7,
            /* First column of the span (1-based, inclusive). */
            "column_start": 15,
            /* Last line of the span (1-based, inclusive). */
            "line_end": 7,
            /* Last column of the span (1-based, exclusive). */
            "column_end": 22
        }
    ]
}
```


Example output
--------------

```
{"message":"this pattern of type\n  Bool\ndoes not cover value\n  false","code":"M0145","level":"warning","spans":[{"file":"example.mo","line_start":2,"column_start":7,"line_end":2,"column_end":11}]}
{"message":"literal of type\n  Text\ndoes not have expected type\n  Nat","code":"M0050","level":"error","spans":[{"file":"example.mo","line_start":5,"column_start":15,"line_end":5,"column_end":22}]}
```

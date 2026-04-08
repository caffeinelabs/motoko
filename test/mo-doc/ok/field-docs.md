# field-docs
A module for testing field doc comments.

## Type `Point`
``` motoko no-repl
type Point = { x : Float; y : Float; name : Text }
```

A record type with documented fields.

`x : Float`

The X coordinate.

`y : Float`

The Y coordinate.

## Type `Shape`
``` motoko no-repl
type Shape = {#circle : Float; #rect : (Float, Float); #dot}
```

A variant type with documented tags.

`#circle : Float`

A circle with a given radius.

`#rect : (Float, Float)`

A rectangle with width and height.

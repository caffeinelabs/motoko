---
title: "Object declarations"
description: "In Motoko, records and objects are both used to group related values using named fields."
sidebar:
  order: 3
---

In Motoko, records and objects are both used to group related values using named fields. Record and objects have the same types, but differ in the way they are created and used. The types of record and objects are both described using object types, which are unordered sequences of named fields describing the content and mutability of each public field. Both record and object fields can be accessed by either dot notation or by pattern matching on the names of the fields.

While records expressions are ideal for lightweight data representation, objects expressions are more verbose. Object expressions can define full objects in the sense of object-oriented programming where an object is a collection of named fields and methods acting on private state. In Motoko, the private declarations define the encapsulated state, while the public definitions define the object's visible members.

Record and object both use the `var` keyword to define [mutable](./variable-declarations.md) and declarations. Both records and objects support `and` and `with` for merging and updating object fields to create new records and objects.

**Record expressions** are used to construct simple data structures that consist of named fields holding values. The fields can be mutable or immutable. The fields of a record cannot refer to each other by name and are mainly used to store plain data, like the records in a database.

Record values can only be declared using `let`:

```motoko no-repl

let Motoko = {
  name : Text = "Motoko";
  var age : Nat = 25;
};
```

All fields of a record are accessible by the [dot notation](https://en.wikipedia.org/wiki/Object-oriented_programming) or by pattern matching on the structure of the record:

```motoko no-repl
// Dot notation
Motoko.age += 1; // Updates age
Motoko.name; // "Motoko"

// Pattern matching
let {name} = Motoko;
name;
```

Object expressions are more general than records. An object is constructed using the `object` keyword followed by a block of field declarations. Each field declaration is a visibility modifier, `public` or `private`, followed by a declaration. Public declarations become accessible as fields of the object, while the private declarations are hidden. Private fields do not appear in the type of the object. Object field declarations can refer to each other in ways that record fields cannot.

## Declaring an object using `let`

In Motoko, the primitive form of object declaration uses `let` to name an object expression. For example, an object might represent a Motoko user profile:

```motoko no-repl
let Motoko = object {
  public let name = "Motoko";
  private var age = 6;

  public func greet() : Text {
    "Hello, " # name;
  };
  public func birthday() : Text{
    age += 1;
    "It's my birthday, I'm now " # debug_show(age) # " years old!"
  }
};
```

This defines an object with three public members, the field `name` and the methods `greet()` and `birthday()`. The mutable field `age` is private, ensuring it can only be increased and read with calls to `birthday()`.

## Declaring an object using an object declaration

Motoko also supports object declarations, which stress the definition of an object by using the `object` keyword in place of `let`:

```motoko no-repl

object Motoko = {
  public let name = "Motoko";
  var age = 6;

  public func greet() : Text {
    "Hello, " # name;
  };
  public func birthday() : Text{
    age += 1;
    "It's my birthday, I'm now " # debug_show(age) # " years old!"
  }
};
```

Since `private` is the default accessibility for the declarations inside an object, the second form is just an abbreviation (syntactic sugar) for the previous `let`-declared object.

### Accessing object fields

Fields and methods of an object are accessed using [dot notation](https://en.wikipedia.org/wiki/Object-oriented_programming) or pattern matching:

```motoko no-repl
// Dot notation
Motoko.greet() # " " # Motoko.birthday();

// Pattern matching
let {greet=g; birthday} = Motoko;
g() # " " # birthday();
```

The field `age` is private and can't be accessed by dot notation or pattern matching.

## Comparing records and objects

| Feature | Record | Object |
|---------|--------|--------|
| Named fields | Yes | Yes |
| Mutable fields | Yes | Yes |
| Private fields | No | Yes |
| Recursively defined fields | No | Yes |
| Combination (`and`, `with`) | Yes | Yes |



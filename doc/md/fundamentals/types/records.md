---
title: "Records"
description: "Records allow you to group related values using named fields, with each field potentially having a different type."
sidebar:
  order: 5
---

Records allow you to group related values using named fields, with each field potentially having a different type.
Unlike [tuples](./tuples.md), which use positional access, records provide field-based access, improving readability and maintainability.

Records also support **mutable fields**, declared using the `var` keyword.
In contrast, all fields in a tuple are always **immutable**.

## Defining a record

```motoko no-repl
let person = {
   name : Text = "Motoko";
   age : Nat = 25;
};
```

:::note[Inferred types]
Type annotations on immutable record fields are optional, as the compiler can infer their types automatically.
However, for mutable fields (`var`), it's good practice to explicitly declare the type.
Without an explicit annotation, the compiler might infer a more specific type than intended, which can restrict future assignments to broader types.

Example:

```motoko no-repl
let account = { name = "Motoko"; var balance = 0 };
// Inferred as { name : Text; var balance : Nat }
account.balance := -100; //  Rejected, -100 is an Int not a Nat
```

To avoid this issue, annotate the field explicitly:

```motoko no-repl
let account = { name = "Motoko"; var balance : Int = 0 };
account.balance := -100; //  Allowed
```

This recommendation also applies to all `var` declarations, not just record fields.
:::

`person` is a record with two labeled fields, `name` of type [`Text`](https://mops.one/core/docs/Text) and `age` of type [`Nat`](https://mops.one/core/docs/Nat).

The values of the fields are `"Motoko"` and `25` respectively.

The type of the `person` value is `{age : Nat; name : Text}`, or, equivalently,  `{name: Text; age : Nat}`.
Unlike tuples, the order of record fields is immaterial and all record types with the same field names and types are considered equivalent, regardless of field ordering.

## Accessing fields

Fields in a record can be accessed using the dot (`.`) notation.

```motoko no-repl
let person = {
   name : Text = "Motoko";
   age : Nat = 25;
  };

let personName = person.name;  // "Motoko"
let personAge = person.age;    // 25

person;
```

Attempting to access a field that isn't available in the type of the record is a compile-time type error.

## Record mutability

By default, record fields are immutable. To create a mutable field, use `var`.

```motoko no-repl
let person = {
    var name : Text = "Motoko";
    var age : Nat = 25;
};
```

This `person` has type `{var age : Nat; var name : Text}`.

`var name` and `var age` allow the values to be updated later.

```motoko no-repl
let person = {
   var name : Text = "Motoko";
   var age : Nat = 25;
  };

person.name := "Ghost";  // Now person.name is "Ghost"
person.age := 30;      // Now person.age is 30

person
```

Attempting to update an immutable field is a compile-time type error.

## Nested records

Records can contain other records, allowing for hierarchical data structures that maintain organization while ensuring type safety and clarity.

```motoko no-repl
type Address = {
    city : Text;
    street : Text;
    zip : Nat;
};

type Individual = {
    name : Text;
    address : Address;
};

let individual : Individual = {
    name = "Motoko";
    address = { street = "101 Broadway"; city = "New York"; zip = 10001 };
};
```

## Pattern matching on records

Records can be destructured using [`switch`](../control-flow/switch.md), allowing selective extraction of fields. This approach makes accessing deeply nested fields more explicit and readable.

```motoko no-repl
type Address = {
    city : Text;
    street : Text;
    zip : Nat;
};

type Individual = {
    name : Text;
    address : Address;
};

let individual : Individual = {
    name = "Motoko";
    address = { street = "101 Broadway"; city = "New York"; zip = 10001 };
};

let { name; address = { city = cityName }} = individual;
```

The pattern both defines `name` as the contents of eponymous field `individual.name` and `cityName` as the contents of field `individual.address.city`.  Irrelevant fields need not be listed in the pattern.
The field pattern `name` is just shorthand for `name = name`: it binds the contents of the field called `name`, to the variable called `name`.

## Using records in collections

Records are commonly used in arrays and other collections for structured data storage, allowing efficient  data organization and retrieval.

```motoko no-repl
type Product = {
    name : Text;
    price : Float;
};

let inventory : [Product] = [
    { name = "Laptop"; price = 999.99 },
    { name = "Smartphone"; price = 599.99 }
];

let firstProduct : Product = inventory[0];
let productName : Text = firstProduct.name;
```

## Updating records programmatically

Since records are immutable by default, updating a record requires creating a modified copy. Motoko allows combining and extending records using the `and` and `with` keywords.

### Merging records with `and`

The `and` keyword merges multiple records when they have no conflicting fields.

```motoko no-repl
let contact = { email : Text = "motoko@example.com"; };
let person = { name : Text = "Motoko"; age : Nat =  25; };

let profile = { person and contact };

debug_show(profile);
```

`profile` combines `person` and `contact` because they have unique fields. If any field name overlaps, `and` alone is not allowed. Use  the `with` keyword to resolve conflicts.

### Overriding and extending records using `with`

The `with` keyword modifies, overrides, or adds fields when combining records.

```motoko no-repl
let person = { name : Text = "Motoko"; age : Nat =  25; };
// age = 26; updates the existing age field.
// city = "New York" adds a new field to the record.
// city = "New York" adds a new field to the record.
let updatedPerson = { person with age : Nat = 26; city : Text = "New York"; };

debug_show(updatedPerson);
```

:::note

If `person` contained a mutable (`var`) field, `with` must redefine it, preventing aliasing.

:::

### Combining `and` and `with`

```motoko no-repl
let person = {name : Text = "Motoko"; age : Nat = 25};
let contact = {email : Text = "motoko@example.com"};

// profile and contact merge with and since they have unique fields.
// age = 26; updates the age field from profile.
// location = "New York"; adds a new field.
let fullProfile = {
  person and contact with
  age = 26;
  location : Text = "New York";
};
debug_show(fullProfile);
```

## Tuples vs records

Tuples and records both allow grouping values, but they have key differences in structure, mutability, and field access. While tuples provide a compact way to group values, records offer more flexibility for structured data modeling, especially when dealing with complex relationships or named fields.

| Feature         | Tuple                                     | Record                                |
|----------------|------------------------------------------|----------------------------------------|
| Structure      | Ordered collection of values             | Unordered collection of named fields  |
| Projection     | By position (`.n`)                       | By field name (`.fieldName`)          |
| Pattern matching| Complete, using ordered tuple patterns                       | Selective, using unordered record patterns  |
| Mutability     | Immutable after creation                 | Can have mutable fields               |
| Naming         | Fields are anonymous  | Fields are named               |
| Subtyping     | Fields cannot be removed | Fields can be removed |
| Use case       | Positional grouping of related values, e.g. vectors     |  Structured data types        |

Motoko's records support more flexible subtyping than tuples.
With records, subtyping allows fields to be omitted in the subtype (a concept known as **width subtyping**).
In contrast, tuple subtyping requires tuples to have the **same length**, making them less flexible in this regard.

For example, `{x : Int, y : Int, z : Int}` is a subtype of `{x : Int, y : Int}`, but `(Int, Int, Int)` is not a subtype of `(Int, Int)`.


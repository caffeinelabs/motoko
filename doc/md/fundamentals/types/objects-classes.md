---
title: "Objects & classes"
description: "In Motoko, an object is a collection of named fields that hold values."
sidebar:
  order: 6
---

## Objects

In Motoko, an object is a collection of named fields that hold values. These values can be plain data or functions.  Each field can be either **mutable** or **immutable** depending on whether it's declared with `var` or not.

A simple object containing just fields of data is like a record in a database.
Motoko's light-weight [record](./records.md) syntax makes it easy to construct such objects.

When fields contain function values, Motoko objects can represent traditional objects with methods, familiar from object-oriented programming (OOP).  From an OOP perspective, an object is an abstraction, defined by the behavior of its methods. Methods are typically used to modify or observe some encapsulated (i.e. hidden) state of an object.

In addition to the record syntax, Motoko let's you define an object from a block of declarations. The declarations in the block can be `public` or `private`, with `private` the default.
Public declarations become accessible fields of the object, while private declarations remain hidden and inaccessible from outside the object.

```motoko no-repl
object Account {
  var balance : Nat = 1000;

  public  func deposit(amount : Nat) : Nat {
    balance += amount;
    balance
  };
  func withdraw(amount : Nat) : ?Nat {
    if (amount > balance) { return null };
      balance -= amount;
      ?balance
    };
}
```

## Classes

An object declaration just declares a single object. To declare a function that generates objects of a similar type, Motoko offer classes. A class acts as a blueprint for creating multiple objects with independent [state](../actors/state.md).

```motoko no-repl
class Account(initialBalance : Nat) {
  var balance = initialBalance;

  public func deposit(amount : Nat) : async Nat {
    balance += amount;
    balance
  };
  public func withdraw(amount : Nat) : ?Nat {
    if (amount > balance) { return null };
      balance -= amount;
      ?balance
    };
};

// Creating multiple accounts
let account1 = Account(500);
let account2 = Account(1000);
```

### Object classes

An object class defines a blueprint for multiple objects. The above is just short-hand for an `object` class. Motoko also support module and actor classes.

```motoko no-repl
object class Account(initialBalance : Nat) {
  var balance = initialBalance;

  public func deposit(amount : Nat) : Nat {
    balance += amount;
    balance
  };

  public func withdraw(amount : Nat) : ?Nat {
    if (amount > balance) { return null };
      balance -= amount;
      ?balance
    };
}
```

## Object subtyping

Object subtyping allows objects with more fields to be treated as subtypes of objects with fewer fields. This enables flexibility in function arguments and object compatibility.

Objects with fewer fields are more general, while objects with additional fields are subtypes of more general types.

| Specificity level | Description | Relationship |
|-------------------|-------------|--------------|
| Most general | `type BasicAccount = { getBalance : () -> Nat }` | Subtype of `{}` |
| Middle generality | `type StandardAccount = { getBalance : () -> Nat; deposit : Nat -> () }` | Subtype of `BasicAccount` |
| Least general | `type PremiumAccount = { getBalance : () -> Nat; deposit : Nat -> (); withdraw : Nat -> Bool }` | Subtype of `StandardAccount` |

Below are example objects for each account type, demonstrating subtyping in practice:

```motoko no-repl
type BasicAccount = {
  getBalance : () -> Nat;
};

type StandardAccount = {
  getBalance : () -> Nat;
  deposit : Nat -> ();
};

type PremiumAccount = {
  getBalance : () -> Nat;
  deposit : Nat -> ();
  withdraw : Nat -> Bool;
};

object _basicAccount : BasicAccount = {
  public func getBalance() : Nat {
    100
  };
};

object _standardAccount : StandardAccount = {
  var balance = 200;

  public func getBalance() : Nat {
    balance
  };

  public func deposit(amount : Nat) {
    balance += amount;
  };
};

object _premiumAccount : PremiumAccount = {
  var balance = 300;

  public func getBalance() : Nat {
    balance
  };

  public func deposit(amount : Nat) {
    balance += amount;
  };

  public func withdraw(amount : Nat) : Bool {
    if (amount <= balance) {
      balance -= amount;
      true
    } else {
      false
    }
  };
};
```

`BasicAccount` is the most general type of account, because `StandardAccount` and `PremiumAccount` can both be used as `BasicAccount`s.

A function expecting `{ getBalance : () -> Nat }` can accept any of the above, since all contain at least that method. However, a function requiring `{ withdraw : Nat -> Bool }` cannot accept more general types that lack this method.

- In general, if type `T` is a subtype of type `U`, then any value of type `T` also has type `U`. The means that a value of the more specific type `T` can pass as a value of the more general type `U` without any explicit conversion.
- When `T` and `U` are object types, subtyping ensures that `T` provides at least the fields required by `U`, so that any object of type `T` can serve as an object of the more general type `U`.

The more general object type has fewer fields because this places fewer requirements on its values.

`PremiumAccount <: StandardAccount <: BasicAccount`, or equivalently:  `PremiumAccount` is a subtype of `StandardAccount`, which is a subtype of `BasicAccount`.

A function expecting `PremiumAccount` expects `withdraw`, so it cannot accept `basicAccount`.
However, a function expecting `BasicAccount` only needs `getBalance`, so it can accept all three type of objects.

```motoko no-repl
import Debug "mo:core/Debug";
import Nat "mo:core/Nat";

func printBalance(account : { getBalance : () -> Nat }) {
  Debug.print("Balance: " # Nat.toText(account.getBalance()));
};

// Works, because all have getBalance
printBalance(_basicAccount);
printBalance(_standardAccount);
printBalance(_premiumAccount);
```

```motoko no-repl
import Debug "mo:core/Debug";
func withdrawFromAccount(account : { withdraw : Nat -> Bool }) {
  let success = account.withdraw(100);
  Debug.print(if success "Withdrawal successful" else "Insufficient funds");
};

// Works only for premiumAccount, fails for others
withdrawFromAccount(_premiumAccount);

// withdrawFromAccount(_standardAccount); // type error: (missing withdraw)
// withdrawFromAccount(_basicAccount);    // type error: (missing withdraw)
```

[Learn more about subtyping](./subtyping.md).


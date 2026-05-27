---
title: "Variants"
description: "Variant type describe values that take on one of several forms, each labeled with a distinct  tag."
sidebar:
  order: 7
---

Variant type describe values that take on one of several forms, each labeled with a distinct  tag. Unlike [records](./records.md), where all fields exist at once, a value of a variant type holds exactly one of the type's possible values. This makes variants useful for representing mutually exclusive alternatives such as states, enumerations, categories and even trees.

## Defining a variant

```motoko no-repl
type Status = {
  #Active;
  #Inactive;
  #Banned : Text;
};
```

`#Active` and `#Inactive` are constant tags, with an implicit `()` argument, meaning they only store trivial data. `#Banned` carries a [`Text`](https://mops.one/core/docs/Text) value, such as the reason for the ban.

## Assigning variants

To assign a variant value, use one of the defined tags.

```motoko no-repl
let activeUser = #Active;
let bannedUser = #Banned("Violation of rules");
```

## Accessing a variant's value

To work with a variant, use a [`switch`](../control-flow/switch.md) expression to match each possible case.

```motoko no-repl
import Debug "mo:core/Debug";

let activeUser : Status = #Active;
let bannedUser : Status = #Banned("Violation of rules");

func getStatusMessage(status : Status) : Text {
  switch (status) {
    case (#Active) "User is active";
    case (#Inactive) "User is inactive";
    case (#Banned(reason)) "User is banned: " # reason;
    };
};

Debug.print(getStatusMessage(activeUser));
Debug.print(getStatusMessage(bannedUser));
```

## Variants example: traffic lights

To demonstrate variants, consider the following example.

A traffic light cycles between three distinct states:

- Red: Vehicles must stop.
- Yellow: Vehicles should prepare to stop.
- Green: Vehicles may proceed.

Since the traffic light can only be in one of these states at a time, a variant is well-suited to model it. There is no invalid [state](../actors/state.md), as every possible value is explicitly defined. The transitions are controlled and predictable.

### Defining the traffic light state

```motoko no-repl
type TrafficLight = {
  #red;
  #yellow;
  #green;
};
```

### Transitioning between states

A function can define how the traffic light cycles from one [state](../actors/state.md) to the next.

```motoko no-repl
func nextState(light : TrafficLight) : TrafficLight {
    switch (light) {
        case (#red)    #green;
        case (#green)  #yellow;
        case (#yellow) #red;
    }
};
nextState(#red);
```

### Simulating traffic light changes

```motoko no-repl
import Debug "mo:core/Debug";
import Iter "mo:core/Iter";

func nextState(light : TrafficLight) : TrafficLight {
  switch (light) {
    case (#red) #green;
    case (#green) #yellow;
    case (#yellow) #red
  }
};

var light : TrafficLight = #red; // Initial state

for (_ in Iter.range(0, 5)) {
  // Cycle through states
  light := nextState(light);
  Debug.print(debug_show (light))
};
```

## Defining a binary tree type using variants

A binary tree is a data structure where each node has up to two child nodes. A variant can be used to represent this structure since a node can either contain a value with left and right children or be an empty leaf. This tree type is recursive as it refers to itself in its definition.

```motoko no-repl
type Tree = {
  #node : {
    value : Nat;
    left : Tree;
    right : Tree
  };
  #leaf
};
```

This example contains two variants:

1. `#node` contains a value of type [`Nat`](https://mops.one/core/docs/Nat) and two child trees (`left` and `right`).
2. `#leaf` represents an empty node.

### Building the tree

The following example defines a tree with a single root node containing the value `10`. It has two child nodes, `5` and `15`, both of which do not have any children.

```motoko no-repl
let tree : Tree = #node {
  value = 10;
  left = #node {value = 5; left = #leaf; right = #leaf};
  right = #node {value = 15; left = #leaf; right = #leaf}
  };
```

### Tree structure

```md
        10
     /      \
    5       15
```

### Traversing the tree

A tree can be traversed in multiple ways. One common approach is in-order traversal, where nodes are visited in the order:

1. Left subtree
2. Root node
3. Right subtree

The following example recursively traverses the tree in order and prints each value as it is visited.

```motoko no-repl
import Debug "mo:core/Debug";

let tree : Tree = #node {
  value = 10;
  left = #node {value = 5; left = #leaf; right = #leaf};
  right = #node {value = 15; left = #leaf; right = #leaf}
};

func traverseInOrder(t : Tree) {
  switch (t) {
    case (#leaf) {};
    case (#node {value; left; right}) {
      traverseInOrder(left);
      Debug.print(debug_show (value));
      traverseInOrder(right)
    }
  }
};
traverseInOrder(tree);
```

### Using generic types

Currently, the example tree only supports [`Nat`](https://mops.one/core/docs/Nat) values. To allow it to store any type of data, a [generic type](./advanced-types.md#generic-types) can be used. A generic type allows a data structure to work with multiple types by using a placeholder type `T`, which is replaced with a specific type when used.

```motoko no-repl
type Tree<T> = {
    #node : {
      value : T;
      left : Tree<T>;
      right : Tree<T>;
    };
    #leaf;
};
```

With this change, the tree can store any type, such as [`Text`](https://mops.one/core/docs/Text), [`Nat`](https://mops.one/core/docs/Nat), or custom types, making it more flexible and reusable.

## Subtyping

In Motoko, a variant with fewer tags is a subtype of a variant with more tags:

```motoko no-repl
type WorkDay = { #mon; #tues; #wed; #thurs; #fri };
type Day = { #sun; #mon; #tues; #wed; #thurs; #fri; #sat};
```

This means that every `WordDay` is also a `Day` and, for example,  a function on `Day` can also be applied to any `WorkDay`.


//MOC-FLAG --package core $MOTOKO_CORE
import Array "mo:core/Array";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import Option "mo:core/Option";
import List "mo:core/List";
import { Tuple2; Tuple3; Tuple4 } "mo:core/Tuples";
import { type Order } "mo:core/Order";

// --- Helpers (to be moved to mo:core) ---

module Unit {
  public func compare(_ : (), _ : ()) : Order { #equal };
};

module Order {
  public func compareBy<T, K>(
    a : T,
    b : T,
    key : T -> K,
    compare : (implicit : (compare : (K, K) -> Order)),
  ) : Order {
    compare(key(a), key(b));
  };
};

module Variant3 {
  public func compare<A, B, C>(
    v1 : { #a : A; #b : B; #c : C },
    v2 : { #a : A; #b : B; #c : C },
    compareA : (implicit : (compare : (A, A) -> Order)),
    compareB : (implicit : (compare : (B, B) -> Order)),
    compareC : (implicit : (compare : (C, C) -> Order)),
  ) : Order {
    switch (v1, v2) {
      case (#a a1, #a a2) compareA(a1, a2);
      case (#a _, _) #less;
      case (_, #a _) #greater;
      case (#b b1, #b b2) compareB(b1, b2);
      case (#b _, _) #less;
      case (_, #b _) #greater;
      case (#c c1, #c c2) compareC(c1, c2);
    };
  };
};

module Variant4 {
  public func compare<A, B, C, D>(
    v1 : { #a : A; #b : B; #c : C; #d : D },
    v2 : { #a : A; #b : B; #c : C; #d : D },
    compareA : (implicit : (compare : (A, A) -> Order)),
    compareB : (implicit : (compare : (B, B) -> Order)),
    compareC : (implicit : (compare : (C, C) -> Order)),
    compareD : (implicit : (compare : (D, D) -> Order)),
  ) : Order {
    switch (v1, v2) {
      case (#a a1, #a a2) compareA(a1, a2);
      case (#a _, _) #less;
      case (_, #a _) #greater;
      case (#b b1, #b b2) compareB(b1, b2);
      case (#b _, _) #less;
      case (_, #b _) #greater;
      case (#c c1, #c c2) compareC(c1, c2);
      case (#c _, _) #less;
      case (_, #c _) #greater;
      case (#d d1, #d d2) compareD(d1, d2);
    };
  };
};

// --- Types ---

type Status = {
  #pending;
  #inProgress : { assignees : ?List.List<Text> };
  #completed : { completedAt : Nat; score : Nat };
};

type Priority = { #low; #medium; #high; #critical };

type Task = {
  id : Nat;
  var name : Text;
  var priority : Priority;
  var status : Status;
  var description : Text; // not used in comparison
};

// --- Compare functions ---

module Status {
  public func compare(a : Status, b : Status) : Order {
    Order.compareBy(
      a,
      b,
      func(s) {
        switch s {
          case (#pending) #a;
          case (#inProgress { assignees }) (#b assignees);
          case (#completed { completedAt; score }) (#c(completedAt, score));
        };
      },
    );
  };
};

module Priority {
  public func compare(a : Priority, b : Priority) : Order {
    Order.compareBy(
      a,
      b,
      func(p) {
        switch p {
          case (#low) #a;
          case (#medium) #b;
          case (#high) #c;
          case (#critical) #d;
        };
      },
    );
  };
};

module Task {
  public func compare(a : Task, b : Task) : Order {
    Order.compareBy(a, b, func(t) { (t.priority, t.status, t.id, t.name) });
  };
};

module TaskByStatus {
  public func compare(a : Task, b : Task) : Order {
    Order.compareBy(a, b, func(t) { (t.status, t.priority, t.id) });
  };
};

// --- Tests ---

let tasks : [Task] = [
  {
    id = 3;
    var name = "Write docs";
    var priority = #low;
    var status = #pending;
    var description = "...";
  },
  {
    id = 1;
    var name = "Fix crash";
    var priority = #critical;
    var status = #completed { completedAt = 100; score = 5 };
    var description = "...";
  },
  {
    id = 2;
    var name = "Add tests";
    var priority = #high;
    var status = #inProgress {
      assignees = ?List.fromArray<Text>(["Alice", "Bob"]);
    };
    var description = "...";
  },
  {
    id = 5;
    var name = "Review PR";
    var priority = #medium;
    var status = #completed { completedAt = 50; score = 3 };
    var description = "...";
  },
  {
    id = 4;
    var name = "Deploy";
    var priority = #critical;
    var status = #pending;
    var description = "...";
  },
  {
    id = 6;
    var name = "Refactor";
    var priority = #high;
    var status = #inProgress { assignees = null };
    var description = "...";
  },
];

// Two compare functions exist for Task, so we must pass explicitly

let sorted = Array.sort<Task>(tasks, Task.compare);

assert sorted[0].name == "Write docs"; // low
assert sorted[1].name == "Review PR"; // medium
assert sorted[2].name == "Refactor"; // high, inProgress (null assignees)
assert sorted[3].name == "Add tests"; // high, inProgress (?[Alice, Bob])
assert sorted[4].name == "Deploy"; // critical, pending
assert sorted[5].name == "Fix crash"; // critical, completed

let byStatus = Array.sort<Task>(tasks, TaskByStatus.compare);

assert byStatus[0].name == "Write docs"; // pending, low
assert byStatus[1].name == "Deploy"; // pending, critical
assert byStatus[2].name == "Refactor"; // inProgress, null assignees
assert byStatus[3].name == "Add tests"; // inProgress, ?[Alice, Bob]
assert byStatus[4].name == "Review PR"; // completed, medium
assert byStatus[5].name == "Fix crash"; // completed, critical

//SKIP comp

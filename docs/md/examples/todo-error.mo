import Int "mo:core/Int";
import Map "mo:core/Map";
import Time "mo:core/Time";
import Result "mo:core/Result";
import Error "mo:core/Error";

persistent actor Todo {

  type Time = Int;
  type Seconds = Int;

  func secondsBetween(start : Time, end : Time) : Seconds =
    (end - start) / 1_000_000_000;

  public type TodoId = Nat;

  type Todo = { #todo : { text : Text; opened : Time }; #done : Time };
  type TodoMap = Map.Map<TodoId, Todo>;

  var idGen : TodoId = 0;
  let todos : TodoMap = Map.empty();

  private func nextId() : TodoId {
    let id = idGen;
    idGen += 1;
    id
  };

  /// Creates a new todo and returns its id
  public shared func newTodo(txt : Text) : async TodoId {
    let id = nextId();
    let now = Time.now();
    Map.add(todos, Int.compare, id, #todo({ text = txt; opened = now }));
    id
  };

  public shared func markDoneBad(id : TodoId) : async Seconds {
    switch (Map.get(todos, Int.compare, id)) {
      case (?(#todo(todo))) {
        let now = Time.now();
        Map.add(todos, Int.compare, id, #done(now));
        secondsBetween(todo.opened, now)
      };
      case _ { -1 };
    }
  };

  public shared func markDoneOption(id : TodoId) : async ?Seconds {
    switch (Map.get(todos, Int.compare, id)) {
      case (?(#todo(todo))) {
        let now = Time.now();
        Map.add(todos, Int.compare, id, #done(now));
        ?(secondsBetween(todo.opened, now))
      };
      case _ { null };
    }
  };

  public type TodoError = { #notFound; #alreadyDone : Time };

  public shared func markDoneResult(id : TodoId) : async Result.Result<Seconds, TodoError> {
    switch (Map.get(todos, Int.compare, id)) {
      case (?(#todo(todo))) {
        let now = Time.now();
        Map.add(todos, Int.compare, id, #done(now));
        #ok(secondsBetween(todo.opened, now))
      };
      case (?(#done(time))) {
        #err(#alreadyDone(time))
      };
      case null {
        #err(#notFound)
      };
    }
  };

  public shared func markDoneException(id : TodoId) : async Seconds {
    switch (Map.get(todos, Int.compare, id)) {
      case (?(#todo(todo))) {
        let now = Time.now();
        Map.add(todos, Int.compare, id, #done(now));
        secondsBetween(todo.opened, now)
      };
      case (?(#done _)) {
        throw Error.reject("Already done")
      };
      case null {
        throw Error.reject("Not Found")
      };
    }
  };

};

persistent actor TodoCaller {

  type Time = Int;
  type Seconds = Int;

  func secondsBetween(start : Time, end : Time) : Seconds =
    (end - start) / 1_000_000_000;

  public shared func mkTodo() : async Todo.TodoId {
    await Todo.newTodo("Write error handling tutorial")
  };

  public shared func doneTodo1(id : Todo.TodoId) : async Text {
    let seconds = await Todo.markDoneBad(id);
    if (seconds != -1) {
      "Congrats! That took " # Int.toText(seconds) # " seconds.";
    } else {
      "Something went wrong.";
    };
  };

  public shared func doneTodo2(id : Todo.TodoId) : async Text {
    switch (await Todo.markDoneOption(id)) {
      case null {
        "Something went wrong."
      };
      case (?seconds) {
        "Congrats! That took " # Int.toText(seconds) # " seconds."
      };
    };
  };

  public shared func doneTodo3(id : Todo.TodoId) : async Text {
    switch (await Todo.markDoneResult(id)) {
      case (#err(#notFound)) {
        "There is no Todo with that ID."
      };
      case (#err(#alreadyDone(at))) {
        let doneAgo = secondsBetween(at, Time.now());
        "You've already completed this todo " # Int.toText(doneAgo) # " seconds ago."
      };
      case (#ok(seconds)) {
        "Congrats! That took " # Int.toText(seconds) # " seconds."
      };
    };
  };

  public shared func doneTodo4(id : Todo.TodoId) : async Text {
    try {
      let seconds = await Todo.markDoneException(id);
      "Congrats! That took " # Int.toText(seconds) # " seconds.";
    } catch _ {
      "Something went wrong.";
    }
  };

};

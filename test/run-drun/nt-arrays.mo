import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// A row may hold arrays: of scalars, of options of scalars and of records of them, and options of
// arrays. A table with array fields builds, evolve adds an array field as an option to a table with
// rows, and a view may name an array field. fieldTypes lists an array field once, with its
// elements' scalar type, and a field of an array of records once per field of the record, as
// "field.sub". An evolve that changes an array's element type traps and changes nothing
actor {
  type Person = {
    name : Text;
    codes : [Text];
    scores : [?Nat];
    tags : ?[Text];
    history : [{ amount : Nat; since : Int; note : ?Text }]
  };

  let people = NativeTable.empty<Nat, Person>().createIndex(["name"]).build();
  let names : NativeTable.NativeTable<Nat, { name : Text; codes : [Text] }> = people.view();

  type Old = { name : Text; age : Nat };
  type New = { name : Text; age : Nat; codes : ?[Text]; history : ?[{ amount : Nat }] };
  let old = NativeTable.empty<Nat, Old>().build();
  var changed : ?NativeTable.NativeTable<Nat, New> = null;

  public func build() : async () {
    Prim.debugPrint(debug_show (people.size(), people.isEmpty(), names.size(), people.indexes));
    for (name, t) in people.fieldTypes().values() {
      Prim.debugPrint(debug_show (name, t))
    }
  };

  public func change() : async () {
    old.add(1, { name = "Ana"; age = 30 });
    old.add(2, { name = "Bo"; age = 41 });
    let t : NativeTable.NativeTable<Nat, New> = old.evolve();
    changed := ?t;
    Prim.debugPrint(debug_show (t.size(), t.sum("age"), t.fieldTypes()))
  };

  public func otherElement() : async () {
    switch changed {
      case ?t {
        let u : NativeTable.NativeTable<Nat, { name : Text; age : Nat; codes : ?[Nat] }> = t.evolve();
        ignore u
      }
      case null {}
    }
  };

  public func size() : async () {
    switch changed {
      case ?t { Prim.debugPrint(debug_show (t.size(), t.sum("age"))) }
      case null {}
    }
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress build "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress otherElement "DIDL\x00\x00"
//CALL ingress size "DIDL\x00\x00"

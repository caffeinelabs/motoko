import Prim "mo:⛔";

module {
  type OldItem = { id : Nat; name : Text; tag : Text };
  type NewItem = { id : Nat; name : Text; tag : Text; count : Nat };
  type User = { id : Nat; email : Text; active : Bool };
  type Order = { id : Nat; amount : Nat; status : Text };
  type Event = { #added : Nat; #removed : Nat };
  type OldActor = {
    items : [OldItem];
    users : [User];
    orders : [Order];
    events : [Event];
  };
  type NewActor = {
    items : [NewItem];
    users : [User];
    orders : [Order];
    events : [Event];
  };

  public func migration(old : OldActor) : NewActor {
    { old with items = Prim.Array_tabulate<NewItem>(old.items.size(), func i = { old.items[i] with count = 0 }) }
  };
};

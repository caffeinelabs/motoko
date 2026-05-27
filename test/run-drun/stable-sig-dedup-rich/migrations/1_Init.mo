module {
  type Item = { id : Nat; name : Text };
  type User = { id : Nat; email : Text; active : Bool };
  type Order = { id : Nat; amount : Nat; status : Text };
  type Event = { #added : Nat; #removed : Nat };
  type NewActor = {
    items : [Item];
    users : [User];
    orders : [Order];
    events : [Event];
  };

  public func migration(_ : {}) : NewActor {
    { items = []; users = []; orders = []; events = [] }
  };
};

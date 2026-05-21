import Array "mo:core/Array";

persistent actor {
  type OldCard = {
    title : Text;
  };

  type NewCard = {
    title : Text;
    description : Text;
  };

  var map : [(Nat32, OldCard)] = [];

  var newMap : [(Nat32, NewCard)] = Array.map<(Nat32, OldCard), (Nat32, NewCard)>(
    map,
    func(key, { title }) { (key, { title; description = "<empty>" }) },
  );
};

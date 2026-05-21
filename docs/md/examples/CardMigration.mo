// CardMigration.mo
import Array "mo:core/Array";

module CardMigration {
  type OldCard = {
    title : Text;
  };

  type NewCard = {
    title : Text;
    description : Text;
  };

  // our migration function
  public func migration(old : {
      var map : [(Nat32, OldCard)] // old type
    }) :
    {
      var map : [(Nat32, NewCard)] // new type
    } {
    { var map : [(Nat32, NewCard)] =
        Array.map<(Nat32, OldCard), (Nat32, NewCard)>(
          old.map,
          func(key, { title }) { (key, { title; description = "<empty>" }) }) }
  }

}

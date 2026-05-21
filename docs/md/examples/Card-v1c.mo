import {migration} "CardMigration";

(with migration) // Declare the migration function
persistent actor {
  type Card = {
    title : Text;
    description : Text;
  };

  var map : [(Nat32, Card)] = []; // Initialized by migration on upgrade
};

persistent actor {
  type Card = {
    title : Text;
    description : Text;
  };

  var map : [(Nat32, Card)] = [];
};

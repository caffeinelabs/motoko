// Import the necessary modules, including the Random module:
import Random = "mo:core/Random";
import Char = "mo:core/Char";
import Error = "mo:core/Error";

// Define an actor

actor {

  // Create a random number generator using cryptographic entropy:
  transient let random = Random.crypto();

  // Define a stable variable that contains each card as a unicode character:
  var deck : ?[var Char] = ?[var
    '🂡','🂢','🂣','🂤','🂥','🂦','🂧','🂨','🂩','🂪','🂫','🂬','🂭','🂮',
    '🂱','🂲','🂳','🂴','🂵','🂶','🂷','🂸','🂹','🂺','🂻','🂼','🂽','🂾',
    '🃁','🃂','🃃','🃄','🃅','🃆','🃇','🃈','🃉','🃊','🃋','🃌','🃍','🃎',
    '🃑','🃒','🃓','🃔','🃕','🃖','🃗','🃘','🃙','🃚','🃛','🃜','🃝','🃞',
    '🃏'
  ];

  // Define a function to shuffle the cards using the Random API.
  public func shuffle() : async () {
    let ?cards = deck else throw Error.reject("shuffle in progress");
    deck := null;
    var i : Nat = cards.size() - 1;
    while (i > 0) {
      let j = await* random.natRange(0, i + 1);
      let temp = cards[i];
      cards[i] := cards[j];
      cards[j] := temp;
      i -= 1;
    };
    deck := ?cards;
  };

  // Define a function to display the randomly shuffled cards.
  public query func show() : async Text {
    let ?cards = deck else throw Error.reject("shuffle in progress");
    var t = "";
    for (card in cards.values()) {
       t #= Char.toText(card);
    };
    t;
  }

};

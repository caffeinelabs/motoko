type Result<Ok, Err> = { #ok : Ok; #err : Err };

func greetOptional(optionalName : ?Text) : Text {
  switch (optionalName) {
    case (null) { "No name to be found." };
    case (?name) { "Hello, " # name # "!" };
  }
};
assert(greetOptional(?"Dominic") == "Hello, Dominic!");
assert(greetOptional(null) ==  "No name to be found");

func greetResult(resultName : Result<Text, Text>) : Text {
  switch (resultName) {
    case (#err(error)) { "No name: " # error };
    case (#ok(name)) { "Hello, " # name };
  }
};
assert(greetResult(#ok("Dominic")) == "Hello, Dominic!");
assert(greetResult(#err("404 Not Found")) == "No name: 404 Not Found");

import Text "mo:base/Text";
import Debug "mo:base/Debug"

actor {
    public func tryFunction() {

        try {
            func greetOptional(optionalName : ?Text) : Text =
                switch optionalName {
                    case null { "No name to be found." };
                    case (?name) { "Hello, " # name # "!" };
                };
            assert greetOptional(?"Motoko") == "Motoko";
        } finally {
            Debug.print("Finally block executed");
        }
    }
}

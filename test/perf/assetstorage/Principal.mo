/// Module for interacting with Principals (users and canisters).
///
/// Principals are used to identify entities that can interact with the Internet
/// Computer. These entities are either users or canisters.
///
/// Example textual representation of Principals:
///
/// `un4fu-tqaaa-aaaab-qadjq-cai`
///
/// In Motoko, there is a primitive Principal type called `Principal`. As an example
/// of where you might see Principals, you can access the Principal of the
/// caller of your shared function.
///
/// ```motoko no-repl
/// shared(msg) func foo() {
///   let caller : Principal = msg.caller;
/// };
/// ```
///
/// Then, you can use this module to work with the `Principal`.
///
/// Import from the base library to use this module.
/// ```motoko name=import
/// import Principal "mo:base/Principal";
/// ```

import Prim "mo:⛔";
import Blob "Blob";
import Hash "Hash";
module {

  public type Principal = Prim.Types.Principal;

  /// Get the `Principal` identifier of an actor.
  ///
  /// Example:
  /// ```motoko include=import no-repl
  /// actor MyCanister {
  ///   func getPrincipal() : Principal {
  ///     let principal = Principal.fromActor(MyCanister);
  ///   }
  /// }
  /// ```
  public func fromActor(a : actor {}) : Principal = Prim.principalOfActor a;

  /// Convert a `Principal` to its `Blob` (bytes) representation.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let blob = Principal.toBlob(principal); // => \00\00\00\00\00\30\00\D3\01\01
  /// ```
  public func toBlob(p : Principal) : Blob = Prim.blobOfPrincipal p;

  /// Converts a `Blob` (bytes) representation of a `Principal` to a `Principal` value.
  ///
  /// Example:
  /// ```motoko include=import
  /// let blob = "\00\00\00\00\00\30\00\D3\01\01" : Blob;
  /// let principal = Principal.fromBlob(blob);
  /// Principal.toText(principal) // => "un4fu-tqaaa-aaaab-qadjq-cai"
  /// ```
  public func fromBlob(b : Blob) : Principal = Prim.principalOfBlob b;

  /// Converts a `Principal` to its `Text` representation.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// Principal.toText(principal) // => "un4fu-tqaaa-aaaab-qadjq-cai"
  /// ```
  public func toText(p : Principal) : Text = debug_show (p);

  /// Converts a `Text` representation of a `Principal` to a `Principal` value.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// Principal.toText(principal) // => "un4fu-tqaaa-aaaab-qadjq-cai"
  /// ```
  public func fromText(t : Text) : Principal = fromActor(actor (t));

  private let anonymousPrincipal : Blob = "\04";

  /// Checks if the given principal represents an anonymous user.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// Principal.isAnonymous(principal) // => false
  /// ```
  public func isAnonymous(p : Principal) : Bool = Prim.blobOfPrincipal p == anonymousPrincipal;

  /// Hashes the given principal by hashing its `Blob` representation.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// Principal.hash(principal) // => 2_742_573_646
  /// ```
  public func hash(principal : Principal) : Hash.Hash = Blob.hash(Prim.blobOfPrincipal(principal));

  /// General purpose comparison function for `Principal`. Returns the `Order` (
  /// either `#less`, `#equal`, or `#greater`) of comparing `principal1` with
  /// `principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// Principal.compare(principal1, principal2) // => #equal
  /// ```
  public func compare(principal1 : Principal, principal2 : Principal) : {
    #less;
    #equal;
    #greater
  } {
    if principal1 < principal2 {
      #less
    } else if principal1 == principal2 {
      #equal
    } else {
      #greater
    }
  };

  /// Equality function for Principal types.
  /// This is equivalent to `principal1 == principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// ignore Principal.equal(principal1, principal2);
  /// principal1 == principal2 // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `==` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `==`
  /// as a function value at the moment.
  ///
  /// Example:
  /// ```motoko include=import
  /// import Buffer "mo:base/Buffer";
  ///
  /// let buffer1 = Buffer.Buffer<Principal>(3);
  /// let buffer2 = Buffer.Buffer<Principal>(3);
  /// Buffer.equal(buffer1, buffer2, Principal.equal) // => true
  /// ```
  public func equal(principal1 : Principal, principal2 : Principal) : Bool {
    principal1 == principal2
  };

  /// Inequality function for Principal types.
  /// This is equivalent to `principal1 != principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// ignore Principal.notEqual(principal1, principal2);
  /// principal1 != principal2 // => false
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `!=` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `!=`
  /// as a function value at the moment.
  public func notEqual(principal1 : Principal, principal2 : Principal) : Bool {
    principal1 != principal2
  };

  /// "Less than" function for Principal types.
  /// This is equivalent to `principal1 < principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// ignore Principal.less(principal1, principal2);
  /// principal1 < principal2 // => false
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `<` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `<`
  /// as a function value at the moment.
  public func less(principal1 : Principal, principal2 : Principal) : Bool {
    principal1 < principal2
  };

  /// "Less than or equal to" function for Principal types.
  /// This is equivalent to `principal1 <= principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// ignore Principal.lessOrEqual(principal1, principal2);
  /// principal1 <= principal2 // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `<=` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `<=`
  /// as a function value at the moment.
  public func lessOrEqual(principal1 : Principal, principal2 : Principal) : Bool {
    principal1 <= principal2
  };

  /// "Greater than" function for Principal types.
  /// This is equivalent to `principal1 > principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// ignore Principal.greater(principal1, principal2);
  /// principal1 > principal2 // => false
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `>` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `>`
  /// as a function value at the moment.
  public func greater(principal1 : Principal, principal2 : Principal) : Bool {
    principal1 > principal2
  };

  /// "Greater than or equal to" function for Principal types.
  /// This is equivalent to `principal1 >= principal2`.
  ///
  /// Example:
  /// ```motoko include=import
  /// let principal1 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// let principal2 = Principal.fromText("un4fu-tqaaa-aaaab-qadjq-cai");
  /// ignore Principal.greaterOrEqual(principal1, principal2);
  /// principal1 >= principal2 // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `>=` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `>=`
  /// as a function value at the moment.
  public func greaterOrEqual(principal1 : Principal, principal2 : Principal) : Bool {
    principal1 >= principal2
  }
}

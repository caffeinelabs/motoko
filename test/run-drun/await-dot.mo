persistent actor {

   module Nat {
     public func id(self : Nat) : Nat { self };
   };

  public func go () : () {
     ignore (await (async 1)).id() == 1;
  }

}

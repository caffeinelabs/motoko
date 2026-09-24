import Mixin "mixins/Counter";
// check system func inspect domain includes mixin methods
// NB: generates odd warning about unused fields counter and increment
actor {
  include Mixin(0);

  system func inspect(
   _ :
   {arg : Blob; caller : Principal;
     msg : {#increment : () -> ()} // from Mixin
   }
   ) : Bool { true };

};


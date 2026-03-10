//MOC-FLAG -A=M0194
persistent actor {

 class C() = this {
   public func m() : async () {
     label l {
       for (l in [0, 1, 2, 3].values()) {
         ignore l == 0;
         break l; // label l, not identifier l!
       }
     }
   }
 }

};

//MOC-FLAG -measure-rts-stack
import { errorMessage; debugPrint; setCandidLimits} = "mo:⛔";

actor {
    let expectedMinimumSize = 29_000;
    setCandidLimits<system>{ numerator = 0;
                             denominator = 1;
                             bias = 1_000_000 };
    public func ser() : async () { await go(false) };
    public func deser() : async () { await go(true) };

    public func go(deserialize : Bool) : async () {
        var i = 0;
        type List = {
          #some : ((), List);
          #none
        };
        var l : List = #none;
        var done = false;
        while (not done) {
          try {
            await async {
              var c = 0;
              while (c < 1024) {
                l := (#some ((),l));
                i += 1;
                c += 1
              };
              let b = to_candid(l);

              let o : ?(List) =
               if (deserialize)
                 from_candid(b)
               else null;
              ()
            }
          } catch e {
            debugPrint(errorMessage(e));
            done := true
          }
        };

        assert(i > expectedMinimumSize);

        let b = to_candid(l);
        debugPrint("serialized");

        let _o : ?(List) =
          if (deserialize)
            from_candid(b)
          else null;

        if (deserialize) debugPrint("deserialized");
    }


}
//SKIP run-low
//SKIP run
//SKIP run-ir
//CALL ingress ser 0x4449444C0000
//CALL ingress deser 0x4449444C0000

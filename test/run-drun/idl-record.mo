//MOC-FLAG -A=M0194
import Prim "mo:⛔";
// This test checks that the IDL decoder can
// do the subtyping from received many-field record
// to a double-field one (field names are in hash order)
actor {
    public query func pair(o : (Text, Int)) : async () {
     switch o {
       case (content, num) Prim.debugPrint ("ok: " # debug_show num);
     }
  };
  public query func record(o : {content: Text; value : Int}) : async () {
     switch o {
       case {content} Prim.debugPrint ("ok: " # content);
     }
  };
  public query func record1(o : {value : Int; byte : Int8}) : async () {
     switch o {
       case {byte} Prim.debugPrint ("ok: " # debug_show byte);
     }
  };
  public query func record2(o : {content: Text; value : Int}, follower : Int8) : async Int8 {
     switch o {
       case {content} { Prim.debugPrint ("ok: " # " " # content # " " # debug_show follower); follower };
     }
  };
  public query func record3(o : {content: Text; value : Int; extra : Nat }) : async () {
     switch o {
       case {content} Prim.debugPrint ("ok: " # content);
     }
  };
}

//CALL query pair 0x4449444C016C020071017C010004486579212A
//CALL query record 0x4449444C016C02b99adecb0171f1fee18d037C010004486579212A
// SKIPPED
// CALL query record1 0x4449444C016C0388be8c890477b99adecb0171f1fee18d037C010004486579212A19
//  needs to jump over redundant `content` field
//CALL query record1 0x4449444C016C03b99adecb0171f1fee18d037C88be8c890477010004486579212A19
//  needs to jump over redundant trailing `byte` field
//CALL query record2 0x4449444C016C03b99adecb0171f1fee18d037C88be8c89047702007704486579212A1819
//  missing field
//CALL query record3 0x4449444C016C02b99adecb0171f1fee18d037C010004486579212A

//SKIP run
//SKIP run-ir
//SKIP run-low

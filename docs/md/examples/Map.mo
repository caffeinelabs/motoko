import VarArray "mo:core/VarArray";
import Buckets "Buckets";

persistent actor Map {

  let n = 8; // number of buckets

  type Key = Nat;
  type Value = Text;

  type Bucket = Buckets.Bucket;

  let buckets : [var ?Bucket] = VarArray.repeat(null, n);

  public func get(k : Key) : async ?Value {
    switch (buckets[k % n]) {
      case null null;
      case (?bucket) await bucket.get(k);
    };
  };

  public func put(k : Key, v : Value) : async () {
    let i = k % n;
    let bucket = switch (buckets[i]) {
      case null {
        let b = await Buckets.Bucket(n, i); // dynamically install a new Bucket
        buckets[i] := ?b;
        b;
      };
      case (?bucket) bucket;
    };
    await bucket.put(k, v);
  };

};

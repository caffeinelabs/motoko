import Prim "mo:prim";
import Buffer "buffer";

module {
  public type Iter<T> = { next : () -> ?T };

  public class HashSet<T>(equal : (T, T) -> Bool, hash : T -> Nat) {
    let initialSize = 1024;
    let occupationThresholdPercentage = 85;
    let growthFactor = 2;
    var table = Prim.Array_init<Buffer.Buffer<T>>(initialSize, Buffer.Buffer<T>(0));
    var count = 0;

    public func size() : Nat {
      count;
    };

    public func add(value : T) {
      let index = hash(value) % table.size();
      let collisionList = table[index];
      if not Buffer.contains(collisionList, value, equal) {
        collisionList.add(value);
        count += 1;
        if count * 100 / table.size() > occupationThresholdPercentage {
          grow();
        };
      };
    };

    func grow() {
      let oldValues = values();
      let oldCount = count;
      table := Prim.Array_init<Buffer.Buffer<T>>(table.size() * growthFactor, Buffer.Buffer<T>(0));
      count := 0;
      for value in oldValues {
        add(value);
      };
      assert (count == oldCount);
    };

    public func contains(value : T) : Bool {
      let index = hash(value) % table.size();
      let collisionList = table[index];
      Buffer.contains(collisionList, value, equal);
    };

    public func values() : Iter<T> {
      let combined = Buffer.Buffer<T>(0);
      for collisionList in table.values() {
        for value in collisionList.values() {
          combined.add(value);
        };
      };
      combined.values();
    };
  };
};

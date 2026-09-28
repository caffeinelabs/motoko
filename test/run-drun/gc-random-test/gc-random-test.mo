import Prim "mo:prim";
import Random "random";
import Values "values";
import Types "types";
import HashSet "hash-set";
import Stack "stack";

module {
  public class GCRandomTest() {
    let seed = 4711;
    let random = Random.Random(seed);
    let types = Types.allocationTypes();

    var root : Values.RandomValue = #none;

    var allocationCount = 0;
    let allocationPerType = Prim.Array_init<Nat>(types.size(), 0);

    func maxAlive() : Nat {
      assert (allocationCount > 0);
      let averageObjectSize = Prim.rts_total_allocation() / allocationCount;
      (Prim.rts_heap_size() + averageObjectSize - 1) / averageObjectSize;
    };

    func randomAllocate() {
      Prim.debugPrint("Random allocation");
      let index = random.next() % types.size();
      let factory = types[index];
      let canAllocate = switch factory.allocationLimit {
        case null { true }
        case ?limit { allocationPerType[index] < limit }
      };
      if canAllocate {
        let randomValue = factory.instantiate(random);
        allocationCount += 1;
        allocationPerType[index] += 1;
        append(randomValue);
      };
    };

    func append(value : Values.RandomValue) {
      let visited = HashSet.HashSet<Values.RandomValue>(Types.isIdentical, Types.toHash);
      let pending = Stack.Stack<Values.RandomValue>();
      switch root {
        case #none {
          root := value;
          return;
        }
        case _ { pending.push(root) }
      };
      while not pending.isEmpty() {
        let current = pending.pop();
        if not visited.contains(current) {
          visited.add(current);
          let currentType = Types.getType(current);
          if currentType.append(current, value) {
            return;
          };
          let references = currentType.readReferences(current);
          for next in references.values() {
            pending.push(next);
          };
        };
      };
      randomWrite(value);
    };

    func randomWrite(value : Values.RandomValue) {
      var randomSteps = random.next() % maxAlive();
      if Values.isNone(root) or randomSteps == 0 {
        root := value;
        return;
      };
      var current = root;
      loop {
        randomSteps -= 1;
        let currentType = Types.getType(current);
        let references = currentType.readReferences(current);
        if references.size() == 0 or randomSteps == 0 {
          currentType.randomWrite(random, current, value);
          return;
        };
        current := references[random.next() % references.size()];
      };
    };

    func randomRead() : Values.RandomValue {
      var randomSteps = random.next() % maxAlive();
      if Values.isNone(root) or randomSteps == 0 {
        return root;
      };
      var current = root;
      loop {
        randomSteps -= 1;
        let currentType = Types.getType(current);
        let references = currentType.readReferences(current);
        if references.size() == 0 or randomSteps == 0 {
          return current;
        };
        current := references[random.next() % references.size()];
      };
    };

    func randomChange() {
      Prim.debugPrint("Random change");
      randomWrite(randomRead());
    };

    func randomCheck() {
      Prim.debugPrint("Random check");
      let value = randomRead();
      Types.getType(value).randomCheck(random, value);
    };

    var totalSteps = 0;

    public func run(steps : Nat) : async () {
      Prim.debugPrint("GC Random Test");
      var count = 0;
      while count < steps {
        totalSteps += 1;
        Prim.debugPrint("Step " # debug_show (totalSteps));
        randomAllocate();
        await async {};
        randomChange();
        await async {};
        randomCheck();
        await async {};
        count += 1;
      };
    };
  };
};

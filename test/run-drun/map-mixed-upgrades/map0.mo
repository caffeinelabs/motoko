// Provides provenance for the committed classical `old.wasm` fixture used by
// `map-mixed-upgrades.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:⛔";
import Cycles = "../cycles/cycles";
import Lib "node0";

// A naive, distributed map from Nat to Text.
// Illustrates dynamic installation of imported actor classes.
// Uses a fixed number of nodes, dynamically installed on demand
// .. and upgraded with a call to upgradeNodes() (without data loss)

actor a {

  type Key = Nat;
  type Value = Text;

  // Number of Nodes
  let n = 8;

  type Node = Lib.Node;

  stable let savedNodes : [var ?(actor{})] = Prim.Array_init(n, null);

  let nodes : [var ?Node] = Prim.Array_init(n, null);

  // Would be nice if these were both tail calls on the platform
  public func lookup(k : Key) : async ?Value {
    switch (nodes[k % n]) {
      case null null;
      case (?node) await node.lookup(k);
    };
  };

  public func insert(k : Key, v : Value) : async () {
    let i = k % n;
    let node = switch (nodes[i]) {
      case null {
        Cycles.add(2_000_000_000_000);
        let n = await Lib.Node(i); // dynamically install a new Node
        nodes[i] := ?n;
        n;
      };
      case (?node) node;
    };
    await node.insert(k, v);
  };

  system func preupgrade () {
     for (i in nodes.keys()) {
       savedNodes[i] := nodes[i];
     }
  };

  public func upgradeNodes() : async () {
    for(i in savedNodes.keys()) {
       switch (savedNodes[i]) {
         case null {};
         case (?n) {
           nodes[i] :=
             ? (await (system Lib.Node)(#upgrade n)(i)); // upgrade!
         }
       }
    }
  };

  public func upgradeNodesKeepMainMemory() : async () {
    for(i in savedNodes.keys()) {
       switch (savedNodes[i]) {
         case null {};
         case (?n) {
           nodes[i] :=
             ? (await (system Lib.Node)(#upgrade_with_persistence { wasm_memory_persistence = #keep; canister = n })(i)); // upgrade!
         }
       }
    }
  };

  public func upgradeNodesReplaceMainMemory() : async () {
    for(i in savedNodes.keys()) {
       switch (savedNodes[i]) {
         case null {};
         case (?n) {
           nodes[i] :=
             ? (await (system Lib.Node)(#upgrade_with_persistence { wasm_memory_persistence = #replace; canister = n })(i)); // upgrade!
         }
       }
    }
  };

  stable var k = 0;
  // add 2 next keys on each call
  public func go() : async () {
    // To get lots of cycles in drun
    if (Cycles.balance() == 0)
      await Cycles.provisional_top_up_actor(a, 100_000_000_000_000);

    var i = 0;
    while (i < 2) {
      k += 1;
      let t = debug_show(k);
      assert (null == (await lookup(k)));
      await insert(k, t);
      assert (?t == (await lookup(k)));
      i += 1;
    };
  };

};

/// `size()` of a core Map and of an IndexedMap (3 secondary indexes),
/// 10^5 rows: `empty_x1000` is the loop alone (1,000 iterations adding a constant),
/// `*_x1000` the same loop calling `size()` (divide the difference by 1,000).
/// Query context: `mops bench oqlbars_size --query`.
import Bench      "mo:bench";
import Map        "mo:core/Map";
import Nat        "mo:core/Nat";
import IndexedMap "../src/IndexedMap";
import _NatValue "../src/NatValue";
import _TextValue "../src/TextValue";
import _RecordValue "../src/RecordValue";
import OQL        "../src";

module {
  type L = { id : Nat; ts : Nat; kind : Text; amount : Nat };
  func mkL(k : Nat) : L = { id = k; ts = k; kind = if (k % 10 == 0) "burn" else if (k % 3 == 0) "mint" else "xfer"; amount = (k * 7) % 1000 };
  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: size()");
    bench.description("1,000 size() calls in a loop, against the loop alone.");
    let m = Map.empty<Nat, L>();
    let im = IndexedMap.new<Nat, L>([("ts", #ordered), ("kind", #hash), ("amount", #ordered)]);
    var k = 0;
    while (k < 100_000) { m.add(Nat.compare, k, mkL(k)); im.put(k, mkL(k)); k += 1 };
    var sink = 0;
    bench.rows(["empty_x1000", "size_map_x1000", "size_im_x1000"]);
    bench.cols(["100000"]);
    bench.runner(func (row, _c) {
      var i = 0;
      switch row {
        case "empty_x1000" { while (i < 1000) { sink += 1; i += 1 } };
        case "size_map_x1000" { while (i < 1000) { sink += m.size(); i += 1 } };
        case _ { while (i < 1000) { sink += im.size(); i += 1 } };
      };
    });
    bench;
  };
};

/// a point read by primary key through the executor (`where id = k`),
/// heap IndexedMap and columnar Table, rows L (4 fields; oqlbars_pointr.bench.mo: rows R).
///   *_heap_noidx : IndexedMap with its usual secondary indexes, none on `id`
///                  (the planner serves only declared indexes, so it scans)
///   *_heap_idx   : the same with an extra `id #hash` index
///   *_table      : OQL.Table, `id` is the store position (entityWith PK)
/// k = n / 2. Query context: `mops bench oqlbars_point --query`.
import Bench      "mo:bench";
import Array      "mo:core/Array";
import Nat        "mo:core/Nat";
import Nat8       "mo:core/Nat8";
import Nat32      "mo:core/Nat32";
import Principal  "mo:core/Principal";
import Prim       "mo:prim";
import IndexedMap "../src/IndexedMap";
import Table      "../src/Table";
import _NatValue "../src/NatValue";
import _IntValue "../src/IntValue";
import _TextValue "../src/TextValue";
import _BoolValue "../src/BoolValue";
import _PrincipalValue "../src/PrincipalValue";
import _RecordValue "../src/RecordValue";
import Entity     "../src/Entity";
import Executor   "../src/Executor";
import Query      "../src/Query";
import Registry   "../src/Registry";
import OQL        "../src";

module {
  type L = { id : Nat; ts : Nat; kind : Text; amount : Nat };
  type R = { id : Nat; ts : Int; amount : Nat; kind : Text; owner : Principal; note : Text; flag : Bool };

  func kindOf(k : Nat) : Text = if (k % 10 == 0) "burn" else if (k % 3 == 0) "mint" else "xfer";
  func mkL(k : Nat) : L = { id = k; ts = k; kind = kindOf(k); amount = (k * 7) % 1000 };
  func mkOwners() : [Principal] = Array.tabulate<Principal>(1_000, func i {
    Prim.principalOfBlob(Prim.arrayToBlob(Array.tabulate<Nat8>(29, func j = Nat8.fromNat((i * 31 + j * 7 + 3) % 256))))
  });
  func mkNotes() : [Text] = Array.tabulate<Text>(51, func i {
    var t = ""; var j = 0;
    while (j < 10 + i) { t #= Prim.charToText(Prim.nat32ToChar(Nat32.fromNat(97 + (i + j) % 26))); j += 1 };
    t
  });
  func mkR(owners : [Principal], notes : [Text], k : Nat) : R = { id = k; ts = 1_760_000_000_000_000_000 + k * 1_000; amount = (k * 7) % 1_000; kind = kindOf(k); owner = owners[k % 1_000]; note = notes[k % 51]; flag = k % 2 == 0 };

  let unrestricted : Executor.Access = func (_ : OQL.Decl) : OQL.Access = #unrestricted;

  type Regs = { no : Registry.Registry; idx : Registry.Registry; tab : Registry.Registry };
  func build(n : Nat) : Regs {
    let ml = IndexedMap.new<Nat, L>([("ts", #ordered), ("kind", #hash), ("amount", #ordered)]);
    let mli = IndexedMap.new<Nat, L>([("id", #hash), ("ts", #ordered), ("kind", #hash), ("amount", #ordered)]);
    let tl = Table.new([("ts", #nat), ("kind", #text), ("amount", #nat)], [("ts", #ordered), ("kind", #hash), ("amount", #ordered)]);
    var k = 0;
    while (k < n) { let l = mkL(k); ml.put(k, l); mli.put(k, l); ignore tl.append(l); k += 1 };
    { no = Registry.build([ml.entity("l", "L", "id").build()]); idx = Registry.build([mli.entity("l", "L", "id").build()]); tab = Registry.build([Table.entityWith(tl, "l", "L", "id").build()]) }
  };

  public func init() : Bench.Bench {
    let bench = Bench.Bench();
    bench.name("Baseline: point read by primary key through the executor, rows L");
    bench.description("where id = n/2; heap without and with an id index; Table (id = position).");
    let r1k = build(1_000); let r10k = build(10_000); let r100k = build(100_000);
    func rFor(col : Text) : Regs = switch col { case "1000" r1k; case "10000" r10k; case _ r100k };
    func nFor(col : Text) : Nat = switch col { case "1000" 1_000; case "10000" 10_000; case _ 100_000 };
    func q(k : Nat) : Query.Query = { start = "l"; where_ = ?(#eq(["id"], #nat(k))); groupBy = []; aggregate = []; orderBy = []; offset = null; limit = null; select = null };
    // correctness: each way returns one row
    for (c in ["1000", "10000", "100000"].values()) {
      let rs = rFor(c); let k = nFor(c) / 2;
      for (reg in [rs.no, rs.idx, rs.tab].values()) assert Executor.runWith(reg, q(k), unrestricted).rows.size() == 1;
    };
    var sink = 0;
    bench.rows(["L_heap_noidx", "L_heap_idx", "L_table"]);
    bench.cols(["1000", "10000", "100000"]);
    bench.runner(func (row, col) {
      let rs = rFor(col);
      let reg = switch row { case "L_heap_noidx" rs.no; case "L_heap_idx" rs.idx; case _ rs.tab };
      sink += Executor.runWith(reg, q(nFor(col) / 2), unrestricted).rows.size();
    });
    bench;
  };
};

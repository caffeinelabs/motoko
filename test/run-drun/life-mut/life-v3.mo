import P = "mo:⛔";
import Region = "../stable-region/Region";

actor Life {

  stable let r = Region.new();

  object Random {
    var state = 1;
    public func next() : Bool {
      state := (123138118391*state + 133489131) % 9999;
      (state % 2 == 0)
    };
  };

  class below(u : Nat) {
    var i = 0;
    public func next() : ?Nat { if (i >= u) null else {let j = i; i += 1; ?j} };
  };

  func readBitV2(bits : [var Nat64], index : Nat) : Bool {
    let bit = P.natToNat64(index);
    let mask : Nat64 = 1 << (bit % 64);
    (bits[P.nat64ToNat(bit >> 6)] & mask) == mask
  };

  func readBit(offset : Nat64, index : Nat) : Bool {
    let bit = P.natToNat64(index);
    let mask : Nat64 = 1 << (bit % 64);
    (Region.loadNat64(r, offset + ((bit >> 6) * 8)) & mask) == mask
  };

  func writeBit(offset : Nat64, index : Nat, v : Bool) {
    let bit = P.natToNat64(index);
    let mask : Nat64 = 1 << (bit % 64);
    let i = (bit >> 6) * 8;
    if v {
      Region.storeNat64(r, offset + i, Region.loadNat64(r, offset + i) | mask)
    }
    else {
      Region.storeNat64(r, offset + i, Region.loadNat64(r, offset + i) & ^mask)
    };
    assert (readBit(offset, index) == v);
  };

  type Cell = Bool;

  type State = {
    #v1 : [[var Cell]];
    #v2 : {size : Nat; bits : [var Nat64]};
    #v3 : {size : Nat; offset : Nat64}
  };


  func ensureMemory(offset : Nat64) {
      let pagesNeeded = ((offset + 65535) / 65536) - Region.size(r);
      if (pagesNeeded > 0) {
        assert (Region.grow(r, pagesNeeded) != 0xFFFF_FFFF_FFFF_FFFF)
      };
  };

  class Grid(index : Nat, state : State) {

    let (n : Nat, offset) =
      switch state {
        case (#v1 css) {
          let n = css.size();
          let len = (n * n) / 64 + 1;
          let offset : Nat64 = P.natToNat64(index * len * 8);
          ensureMemory(offset + P.natToNat64(len) * 8);
          for (i in css.keys()) {
            for (j in css[i].keys()) {
              writeBit(offset, i * n + j, css[i][j]);
            };
          };
          (n, offset)
        };
        case (#v2 {size; bits}) {
          let len = (size * size) / 64 + 1;
          let offset : Nat64 = P.natToNat64(index * len * 8);
          ensureMemory(offset + P.natToNat64(len) * 8);
          for (i in below(size)) {
            for (j in below(size)) {
               let k = i * size + j;
               writeBit(offset, k, readBitV2(bits, k));
            }
          };
          (size, offset)
        };
        case (#v3 {size; offset}) {
          let len = (size * size) / 64 + 1;
          let newoffset : Nat64 = P.natToNat64(index * len * 8);
          ensureMemory(newoffset + P.natToNat64(len) * 8);
          if (offset != newoffset) {
            for (i in below(size)) {
              for (j in below(size)) {
                let k = i * size + j;
                writeBit(newoffset, k, readBit(offset, k));
              }
            };
          };
          (size, newoffset)
        };
      };

    public func size() : Nat { n };

    public func get(i : Nat, j : Nat) : Cell {
      readBit(offset, i * n + j);
    };

    public func set(i : Nat, j : Nat, v : Cell) {
      writeBit(offset, i * n + j, v);
    };

    func pred(i : Nat) : Nat { (n + i - 1) % n };

    func succ(i : Nat) : Nat { (i + 1) % n };

    func count(i : Nat, j : Nat) : Nat { if (get(i, j)) 1 else 0 };

    func living(i : Nat, j : Nat) : Nat {
      count(pred i, pred j) + count(pred i, j) + count(pred i, succ j) +
      count(     i, pred j)                    + count(     i, succ j) +
      count(succ i, pred j) + count(succ i, j) + count(succ i, succ j)
    };

    func nextCell(i : Nat, j : Nat) : Cell {
      let l : Nat = living(i, j);
      if (get(i, j))
        l == 2 or l == 3
      else
        l == 3;
    };

    public func next(dst : Grid) {
      for (i in below(n)) {
        for (j in below(n)) {
          dst.set(i, j, nextCell(i, j));
        };
      };
    };

    public func toState() : State {
      #v3 { size = n; offset = offset }
    };

    public func toText() : Text {
      var t = "\n";
      for (i in below(n)) {
        for (j in below(n)) {
          t #= if (get(i, j)) "O" else " ";
        };
        t #= "\n";
      };
      t
    };
  };

  func newState(index : Nat, size : Nat) : State {
    let len = (size * size) / 64 + 1;
    let offset : Nat64 = P.natToNat64(index * len * 8);
    ensureMemory(offset + P.natToNat64(len) * 8);
    for (i in below(len)) {
      var word : Nat64 = 0;
      for (j in below(64)) {
        let bit : Nat64 = if (Random.next()) 0 else 1;
        word |= bit;
        word <<= 1;
      };
      Region.storeNat64(r, offset + P.natToNat64(i) * 8, word );
    };
    #v3 { size; offset};
  };

  stable var state : State = newState(0, 32);

  flexible var src = Grid(0, state);
  flexible var dst = Grid(32, newState(32, src.size()));

  func update(c : Nat) {
    var i = c;
    while (i > 0) {
      src.next(dst);
      let temp = src;
      src := dst;
      dst := temp;
      i -= 1;
    };
  };

  system func preupgrade() {
    state := src.toState();
  };

  system func postupgrade() {
    P.debugPrint("upgraded!");
  };

  public func advance(n : Nat) : async () {
     update(n);
  };

  public query func show() : async () {
     P.debugPrint(src.toText());
  };

};

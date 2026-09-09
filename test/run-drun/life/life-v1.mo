import P = "mo:⛔";

actor Life {

  transient object Random {
    var state = 1;
    public func next() : Bool {
      state := (123138118391*state + 133489131) % 9999;
      (state % 2 == 0)
    };
  };

  type Cell = Bool;

  type State = {
     #v1 : [[Cell]];
  };

  class Grid((#v1 state) : State) {

    let n = state.size();

    public func size() : Nat { n };

    let grid = P.Array_tabulate(n, func (i : Nat) : [var Cell] {
      let a = P.Array_init<Bool>(n, false);
      let si = state[i];
      assert (si.size() == n);
      for (j in si.keys()) {
        a[j] := si[j];
      };
      a
    });

    public func get(i : Nat, j : Nat) : Cell { grid[i][j] };

    public func set(i : Nat, j : Nat, v : Cell) { grid[i][j] := v };

    func pred(i : Nat) : Nat { (n + i - 1) % n };

    func succ(i : Nat) : Nat { (i + 1) % n };

    func count(i : Nat, j : Nat) : Nat { if (grid[i][j]) 1 else 0 };

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
      for (i in grid.keys()) {
        for (j in grid[i].keys()) {
          dst.set(i, j, nextCell(i, j));
        };
      };
    };

    public func toState() :  State {
      #v1 (
        P.Array_tabulate<[Cell]>(n,
          func i { P.Array_tabulate<Cell>(n, func j { get(i, j) }) }))
    };

    public func toText() : Text {
      var t = "\n";
      for (i in grid.keys()) {
        for (j in grid[i].keys()) {
          t #= if (get(i, j)) "O" else " ";
        };
        t #= "\n";
      };
      t
    };
  };

  stable var state : State =
    do {
      let n = 32;
      #v1 (
      	 P.Array_tabulate<[Cell]>(n,
           func i { P.Array_tabulate<Cell>(n, func j { Random.next(); }) })
      )
    };

  flexible var src = Grid(state);
  flexible var dst = Grid(state);

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

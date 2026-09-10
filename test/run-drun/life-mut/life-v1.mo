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
     #v1 : [[var Cell]];
  };

  class Grid(#v1 grid : State) {

    let n = grid.size();

    public func size() : Nat { n };

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
      #v1 grid
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

  func newState(size : Nat) : State {
    let size = 32;
      #v1 (
      	 P.Array_tabulate<[var Cell]>(size,
           func i {
             let ai = P.Array_init<Bool>(size, false);
             for (j in ai.keys()) { ai[j] := Random.next() };
             ai })
      )
  };

  stable var state : State = newState(32);

  transient var src = Grid(state);
  transient var dst = Grid(newState(src.size()));

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

import Prim "mo:⛔";
actor {
  // based on https://github.com/dfinity-lab/dapps/blob/75ead35363574f3697e37cd3a0592e51d3253a36/examples/reversi/src/reversi/main.mo
  // with stdlib inlined and board size changed to 8

  // inline parts of stdlib

  class range(x : Nat, y : Nat) {
    var i = x;
    public func next() : ?Nat { if (i > y) null else { let j = i; i += 1; ?j } };
  };

  flexible func unreachable() : None { assert false; loop {} };

  flexible func toText(x : Int) : Text {
    if (x == 0) {
      return "0";
    };

    let isNegative = x < 0;
    var int = if isNegative (-x) else x;

    var text = "";
    let base = 10;

    while (int > 0) {
      let rem = int % base;
      text := (
        switch (rem) {
          case 0 "0";
          case 1 "1";
          case 2 "2";
          case 3 "3";
          case 4 "4";
          case 5 "5";
          case 6 "6";
          case 7 "7";
          case 8 "8";
          case 9 "9";
          case _ unreachable();
        }
      ) # text;
      int := int / base;
    };

    return if isNegative ("-" # text) else text;
  };

  // Color related type and constants
  type Color = Nat;
  flexible let empty : Color = 0;
  flexible let white : Color = 1;
  flexible let black : Color = 2;

  // Dimension of the board
  flexible let N : Nat = 8;

  // Board is NxN array
  type Board = [var Color];
  flexible let Board : Board = Prim.Array_init<Nat>(N * N, empty);

  // Which color should move next
  flexible var next_color : Color = white;

  // Reset the board to initial state
  flexible func init() {
    // Reset to empty board
    for (i in range(0, N * N - 1)) {
      Board[i] := empty;
    };

    // initialize center 4 pieces
    let M = N / 2;
    Board[(M - 1) * N + M - 1] := white;
    Board[M * N + M] := white;
    Board[(M - 1) * N + M] := black;
    Board[M * N + M - 1] := black;

    // White moves first
    next_color := white;
  };

  // External interface to reset the board
  public func reset() : () {
    init();
  };

  public func dimension() : async Nat {
    return N;
  };

  // Render the board into a string
  flexible func render(board : Board) : Text {
    var str = "";
    for (i in range(0, N - 1)) {
      for (j in range(0, N - 1)) {
        if (board[i * N + j] == white) {
          str := str # "O";
        } else if (board[i * N + j] == black) {
          str := str # "*";
        } else {
          str := str # ".";
        };
      };
      str := str # "\n";
    };

    return str;
  };

  // External interface to render the board
  public func board() : async Text {
    return render(Board);
  };

  // Given a color, return its opponent color
  flexible func opponent(color : Color) : Color {
    return (3 - color);
  };

  // Check if a piece of the given color exists on the board using
  // coordinate (i, j) and offset (p, q).
  flexible func exists(board : Board, color : Color, i : Nat, j : Nat, p : Int, q : Int) : Bool {
    let s = i + p;
    let t = j + q;
    if (s < 0 or s >= N or t < 0 or t >= N) {
      return false;
    };
    return (board[Prim.abs(s * N + t)] == color);
  };

  // Check if a piece of the given color eventually exits on the board
  // using coordinate (i, j) and direction (p, q), ignoring opponent colors
  // in between. Return false if the given color is not found before reaching
  // empty cell or board boundary.
  flexible func eventually(board : Board, color : Color, i : Nat, j : Nat, p : Int, q : Int) : Bool {
    if (exists(board, opponent(color), i, j, p, q)) {
      // the abs below is save because its precondition is already checked
      return eventually(board, color, Prim.abs(i + p), Prim.abs(j + q), p, q);
    } else {
      return exists(board, color, i, j, p, q);
    };
  };

  // Flip pieces of opponent color into the given color starting from
  // coordinate (i, j) and along direction (p, q).
  flexible func flip(board : Board, color : Color, i : Nat, j : Nat, p : Int, q : Int) {
    if (exists(board, opponent(color), i, j, p, q)) {
      // the abs below is save because its precondition is already checked
      let s = Prim.abs(i + p);
      let t = Prim.abs(j + q);
      board[s * N + t] := color;
      flip(board, color, s, t, p, q);
    };
  };

  // Calculate all validate positions for a given color by returning
  // a board that has the cells colored.
  flexible func valid_moves(board : Board, color : Color) : Board {
    let next : Board = Prim.Array_init<Nat>(N * N, empty);
    for (i in range(0, N - 1)) {
      for (j in range(0, N - 1)) {
        if (board[i * N + j] == empty) {
          for (p in [-1, 0, 1].values()) {
            for (q in [-1, 0, 1].values()) {
              if (not (p == 0 and q == 0)) {
                if (
                  exists(board, opponent(color), i, j, p, q) and
                  eventually(board, color, i, j, p, q)
                ) {
                  next[i * N + j] := color;
                };
              };
            };
          };
        };
      };
    };
    return next;
  };

  // Set a piece on the board at a given position, and flip all
  // affected opponent pieces accordingly. It requires that the
  // given position is a valid move before this call.
  flexible func set_and_flip(board : Board, color : Color, i : Nat, j : Nat) {
    board[i * N + j] := color;
    for (p in [-1, 0, 1].values()) {
      for (q in [-1, 0, 1].values()) {
        if (not (p == 0 and q == 0)) {
          if (
            exists(board, opponent(color), i, j, p, q) and
            eventually(board, color, i, j, p, q)
          ) {
            flip(board, color, i, j, p, q);
          };
        };
      };
    };
  };

  // Check if the given board is empty.
  flexible func is_empty(board : Board) : Bool {
    for (c in board.values()) {
      if (c != empty) {
        return false;
      };
    };
    return true;
  };

  // Return the white and black counts.
  flexible func score(board : Board) : (Nat, Nat) {
    var wc = 0;
    var bc = 0;
    for (c in board.values()) {
      if (c == white) {
        wc += 1;
      } else if (c == black) {
        bc += 1;
      };
    };
    return (wc, bc);
  };

  // External interface that places a piece of given color at a coordinate.
  // It returns "OK" when the move is valid.
  public func place(color_ : Int, row_ : Int, col_ : Int) : async Text {
    // The casting is necessary because dfx has yet to support Nat on commandline
    let color : Color = Prim.abs(color_);
    let row : Nat = Prim.abs(row_);
    let col : Nat = Prim.abs(col_);

    // Check input validity
    if (row >= N or col >= N) {
      return "Invalid coordinate";
    };
    if (not (color == 1 or color == 2)) {
      return "Invalid piece color, must be either 1 or 2";
    };

    var possible = valid_moves(Board, next_color);

    // If no move is possible, either pass or end game.
    if (is_empty(possible)) {
      next_color := opponent(next_color);
      possible := valid_moves(Board, next_color);
      // If no possible move again, end game
      if (is_empty(possible)) {
        let (wc, bc) = score(Board);
        return ("End Game! Whites = " # toText(wc) # ", Blacks = " # toText(bc));
      } else {
        return "PASS";
      };
    };

    if (next_color != color) {
      return "Wrong color for this move";
    };

    if (possible[row * N + col] != empty) {
      set_and_flip(Board, color, row, col);
      next_color := opponent(color);
      return "OK";
    } else {
      return "Illegal move";
    };

  };

  public func getPerfData() : async () {
    Prim.debugPrint("instructions: " # debug_show (Prim.rts_lifetime_instructions()));
  };

  init();
};

//CALL ingress reset 0x4449444C0000
//CALL ingress board 0x4449444C0000
//CALL ingress place 0x4449444C00037c7c7c010204
//CALL ingress board 0x4449444C0000
//CALL ingress place 0x4449444C00037c7c7c020203
//CALL ingress board 0x4449444C0000
//CALL ingress place 0x4449444C00037c7c7c010402
//CALL ingress board 0x4449444C0000

//CALL ingress getPerfData 0x4449444C0000

//SKIP run
//SKIP run-ir
//SKIP run-low

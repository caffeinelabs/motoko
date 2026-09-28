// testing let-else as last declaration in block
let block1 = do {
  let x = 4 else { assert false; loop { () } };
};

assert 4 == block1;

let block2 = do {
  let (x, y) = (4, 3) else { assert false; loop { () } };
};

assert (4, 3) == block2;

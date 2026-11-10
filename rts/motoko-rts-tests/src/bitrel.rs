use motoko_rts::bitrel::BitRel;

pub unsafe fn test() {
    println!("Testing bitrel ...");

    const K: usize = 48;

    const N: usize = 2 * K * K * 2 / usize::BITS as usize;

    let mut cache: [usize; N] = [0xFFFFFFF; N];

    assert_eq!(usize::BITS, 64);
    for size1 in 0..K {
        for size2 in 0..K {
            let w = BitRel::words(size1, size2);
            let bitrel = BitRel {
                ptr: &mut cache[0],
                end: &mut cache[w],
                size1: size1,
                size2: size2,
            };
            bitrel.init();
            for i in 0..size1 {
                for j in 0..size2 {
                    // initially unvisited
                    assert!(!bitrel.visited(true, i, j)); // co
                    assert!(!bitrel.visited(false, j, i)); // contra

                    // initially related
                    assert!(bitrel.related(true, i, j)); // co
                    assert!(bitrel.related(false, j, i)); // contra

                    // test visiting
                    // co
                    bitrel.visit(true, i, j);
                    assert!(bitrel.visited(true, i, j));
                    // contra
                    bitrel.visit(false, j, i);
                    assert!(bitrel.visited(false, j, i));

                    // test refutation
                    // co
                    bitrel.assume(true, i, j);
                    assert!(bitrel.related(true, i, j));
                    bitrel.disprove(true, i, j);
                    assert!(!bitrel.related(true, i, j));
                    // contra
                    bitrel.assume(false, j, i);
                    assert!(bitrel.related(false, j, i));
                    bitrel.disprove(false, j, i);
                    assert!(!bitrel.related(false, j, i));
                }
            }
        }
    }
}

use motoko_rts::{
    gc::incremental::array_slicing::slice_array,
    memory::alloc_array,
    types::{TAG_ARRAY_I, TAG_ARRAY_M, TAG_ARRAY_S, TAG_ARRAY_SLICE_MIN, TAG_ARRAY_T, Tag, Words},
};

use crate::memory::TestMemory;

pub unsafe fn test() {
    println!("  Testing array slicing...");

    let tags = vec![TAG_ARRAY_I, TAG_ARRAY_M, TAG_ARRAY_T, TAG_ARRAY_S];

    for tag in tags.into_iter() {
        let mut mem = TestMemory::new(Words(1024 * 1024));
        // multiple of slice increment
        test_array_slicing(&mut mem, tag, 4096);
        // odd remainder of slice increment
        test_array_slicing(&mut mem, tag, 3999);
        // small array
        test_array_slicing(&mut mem, tag, 10);
        // empty array
        test_array_slicing(&mut mem, tag, 0);
    }
}

unsafe fn test_array_slicing(mem: &mut TestMemory, tag: Tag, array_length: usize) {
    let array = alloc_array(mem, tag, array_length).as_array();
    let mut last_offset = 0;
    loop {
        let new_offset = slice_array(array);
        assert!(new_offset > last_offset || array.len() == 0 && new_offset == 0);
        last_offset = new_offset;
        if (*array).header.tag == tag {
            break;
        }
        assert!((*array).header.tag >= TAG_ARRAY_SLICE_MIN);
    }
    assert_eq!(array.len(), array_length);
    assert_eq!(last_offset, array.len());
}

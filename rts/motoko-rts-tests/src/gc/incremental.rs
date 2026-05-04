pub mod array_slicing;
pub mod mark_bitmap;
pub mod mark_stack;
pub mod partitioned_heap;
pub mod roots;
pub mod sort;
pub mod time;

pub fn test() {
    println!("Testing incremental GC ...");
    unsafe {
        array_slicing::test();
        mark_bitmap::test();
        mark_stack::test();
        partitioned_heap::test();
        sort::test();
        roots::test();
        time::test();
    }
}

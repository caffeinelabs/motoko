use crate::{
    barriers::allocation_barrier,
    memory::Memory,
    types::{Array, Bytes, TAG_ARRAY_T, Value, Words, size_of},
};

use motoko_rts_macros::ic_mem_fn;

const ITER_BLOB_IDX: usize = 0;

const ITER_POS_IDX: usize = 1;

/// Returns iterator for the given blob
#[ic_mem_fn]
unsafe fn blob_iter<M: crate::memory::Memory>(mem: &mut M, blob: Value) -> Value {
    let iter_ptr = mem.alloc_words(size_of::<Array>() + Words(2));

    // NB. cannot use as_array() here as we didn't write the header yet
    let iter_array = iter_ptr.get_ptr() as *mut Array;
    (*iter_array).header.tag = TAG_ARRAY_T;
    (*iter_array).header.init_forward(iter_ptr);
    (*iter_array).len = 2;

    iter_array.initialize(ITER_BLOB_IDX, blob, mem);
    iter_array.initialize(ITER_POS_IDX, Value::from_scalar(0), mem);

    allocation_barrier(iter_ptr)
}

/// Returns whether the iterator is finished
#[unsafe(no_mangle)]
unsafe extern "C" fn blob_iter_done(iter: Value) -> usize {
    let iter_array = iter.as_array();

    let blob = iter_array.get(ITER_BLOB_IDX);
    let pos = Bytes(iter_array.get(ITER_POS_IDX).get_scalar());

    (pos >= blob.as_blob().len()).into()
}

/// Reads next byte, advances the iterator
#[ic_mem_fn]
unsafe fn blob_iter_next<M: Memory>(mem: &mut M, iter: Value) -> usize {
    let iter_array = iter.as_array();

    let blob = iter_array.get(ITER_BLOB_IDX);
    let pos = iter_array.get(ITER_POS_IDX).get_scalar();

    iter_array.set(ITER_POS_IDX, Value::from_scalar(pos + 1), mem);

    blob.as_blob().get(pos).into()
}

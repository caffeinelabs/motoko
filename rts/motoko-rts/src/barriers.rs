use crate::{memory::Memory, types::Value};

pub unsafe fn init_with_barrier<M: Memory>(_mem: &mut M, location: *mut Value, value: Value) {
    *location = value.forward_if_possible();
}

pub unsafe fn write_with_barrier<M: Memory>(mem: &mut M, location: *mut Value, value: Value) {
    crate::gc::incremental::barriers::write_with_barrier(mem, location, value);
}

pub unsafe fn allocation_barrier(new_object: Value) -> Value {
    crate::gc::incremental::barriers::allocation_barrier(new_object)
}

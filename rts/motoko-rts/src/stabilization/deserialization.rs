mod scan_stack;
pub mod stable_memory_access;

use crate::{
    constants::MB,
    gc::incremental::array_slicing::slice_array,
    memory::Memory,
    stabilization::deserialization::scan_stack::STACK_EMPTY,
    types::{FwdPtr, Tag, Value, NULL_POINTER, TAG_ARRAY_SLICE_MIN, TAG_FWD_PTR},
    visitor::visit_pointer_fields,
};

use self::{scan_stack::ScanStack, stable_memory_access::StableMemoryAccess};

use super::{
    clear_stable_memory,
    graph_copy::{limit::ExecutionMonitor, GraphCopy},
    layout::{deserialize, StableValue},
};

pub struct Deserialization {
    from_space: StableMemoryAccess,
    scan_stack: ScanStack,
    stable_start: u64,
    stable_size: u64,
    stable_root: Option<Value>,
    limit: ExecutionMonitor,
    clear_position: u64,
    /// Heap addresses for the helper GC roots.
    pub dedup_table_address: Value,
    pub migrations_list_address: Value,
}

/// Helper type to pass serialization context instead of closures.
pub struct DeserializationContext<'a, M: Memory> {
    pub deserialization: &'a mut Deserialization,
    pub mem: &'a mut M,
}

impl<'a, M: Memory> DeserializationContext<'a, M> {
    fn new(
        deserialization: &'a mut Deserialization,
        mem: &'a mut M,
    ) -> DeserializationContext<'a, M> {
        DeserializationContext {
            deserialization,
            mem,
        }
    }
}

/// Graph-copy-based deserialization.
/// Usage:
/// ```
/// let deserialization = Deserialization::start(mem, stable_start, stable_size);
/// while !deserialization.is_completed() {
///     deserialization.copy_increment();
/// }
/// ```
/// Note: The deserialized memory is cleared as final process, using an incremental
/// mechanism to avoid instruction limit exceeding.
impl Deserialization {
    /// Start the deserialization, followed by a series of copy increments.
    pub fn start<M: Memory>(
        mem: &mut M,
        stable_start: u64,
        stable_size: u64,
        dedup_table_address: StableValue,
        migrations_list_address: StableValue,
    ) -> Deserialization {
        let from_space = StableMemoryAccess::open(stable_start, stable_size);
        let scan_stack = unsafe { ScanStack::new(mem) };
        let limit = ExecutionMonitor::new();
        let mut deserialization = Deserialization {
            from_space,
            scan_stack,
            stable_start,
            stable_size,
            stable_root: None,
            limit,
            clear_position: stable_start,
            dedup_table_address: NULL_POINTER,
            migrations_list_address: NULL_POINTER,
        };
        let _ = deserialization.start(mem, StableValue::serialize(Value::from_ptr(0)));
        // Load up the heap addresses of the GC helper roots.
        if dedup_table_address != StableValue::from_raw(0) {
            deserialization.dedup_table_address = deserialization.start(mem, dedup_table_address);
        }
        if migrations_list_address != StableValue::from_raw(0) {
            deserialization.migrations_list_address =
                deserialization.start(mem, migrations_list_address);
        }
        deserialization
    }

    pub fn get_stable_root(&self) -> Value {
        self.stable_root.unwrap()
    }

    unsafe fn scan_deserialized<
        'a,
        M: Memory,
        F: Fn(&mut DeserializationContext<'a, M>, Value) -> Value,
    >(
        context: &mut DeserializationContext<'a, M>,
        target_object: Value,
        translate: &F,
    ) {
        debug_assert!(target_object.is_obj());
        visit_pointer_fields(
            context,
            target_object.as_obj(),
            target_object.tag(),
            0,
            |context, field_address| {
                *field_address = translate(context, *field_address);
            },
            |context, _, array| {
                let length = slice_array(array);
                if (*array).header.tag >= TAG_ARRAY_SLICE_MIN {
                    // Push the array back on the stack to visit the next array slice.
                    context
                        .deserialization
                        .scan_stack
                        .push(context.mem, target_object);
                }
                length
            },
        );
    }

    fn stable_end(&self) -> u64 {
        self.stable_start.checked_add(self.stable_size).unwrap()
    }

    fn processed_memory(&self) -> u64 {
        let deserialized_memory = unsafe { deserialized_size() as u64 };
        debug_assert!(self.clear_position >= self.stable_start);
        let cleared_memory = self.clear_position - self.stable_start;
        deserialized_memory + cleared_memory
    }
}

impl GraphCopy<StableValue, Value, u32> for Deserialization {
    fn get_forward_address(&self, stable_object: StableValue) -> Option<Value> {
        let address = stable_object.to_stable_address();
        let tag = self.from_space.read::<Tag>(address);
        match tag {
            TAG_FWD_PTR => {
                let forward_object = self.from_space.read::<FwdPtr>(address);
                Some(forward_object.fwd)
            }
            _ => None,
        }
    }

    fn set_forward_address(&mut self, stable_object: StableValue, target: Value) {
        let address = stable_object.to_stable_address();
        let forward_object = FwdPtr {
            tag: TAG_FWD_PTR,
            fwd: target,
        };
        self.from_space.write(address, &forward_object);
    }

    fn copy<M: Memory>(&mut self, mem: &mut M, stable_object: StableValue) -> Value {
        unsafe {
            let target = deserialize(mem, &mut self.from_space, stable_object);
            if self.stable_root.is_none() {
                self.stable_root = Some(target);
            }
            self.scan_stack.push(mem, target);
            target
        }
    }

    /// Note:
    /// * The deserialized memory may contain free space at a partition end.
    fn scan<M: Memory>(&mut self, mem: &mut M) {
        let target_object = unsafe { self.scan_stack.pop() };
        debug_assert!(target_object != STACK_EMPTY);
        unsafe {
            Self::scan_deserialized(
                &mut DeserializationContext::new(self, mem),
                target_object,
                &|context, original| {
                    let old_value = StableValue::serialize(original);
                    if original.is_non_null_ptr() {
                        context.deserialization.evacuate(context.mem, old_value)
                    } else {
                        original
                    }
                },
            );
        }
    }

    fn scanning_completed(&self) -> bool {
        unsafe { self.scan_stack.is_empty() }
    }

    fn cleanup_completed(&self) -> bool {
        debug_assert!(self.scanning_completed());
        debug_assert!(self.clear_position <= self.stable_end());
        self.clear_position >= self.stable_end()
    }

    fn cleanup(&mut self) {
        // Optimum value according to experimental measurements:
        // Smallest chunk size that does not cause noticeable performance regression.
        // The granularity is still small enough to meet the instruction limit.
        const MAX_CHUNK_SIZE: u64 = MB as u64;
        debug_assert!(!self.cleanup_completed());
        let end = self.stable_end();
        assert!(self.clear_position < end);
        let remainder = end - self.clear_position;
        let chunk = core::cmp::min(MAX_CHUNK_SIZE, remainder);
        clear_stable_memory(self.clear_position, chunk);
        self.clear_position += chunk;
    }

    fn time_over(&mut self) -> bool {
        self.limit.is_exceeded(self.processed_memory())
    }

    fn reset_time(&mut self) {
        self.limit.reset(self.processed_memory());
    }
}

#[cfg(feature = "ic")]
unsafe fn deserialized_size() -> usize {
    crate::memory::ic::partitioned_memory::get_heap_size().as_usize()
}

// Injection point for RTS unit testing.
#[cfg(not(feature = "ic"))]
extern "C" {
    fn deserialized_size() -> usize;
}

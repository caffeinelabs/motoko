pub mod stable_memory_stream;

use crate::{
    memory::Memory,
    stabilization::layout::serialize,
    types::{FwdPtr, Tag, Value, TAG_CLOSURE, TAG_FWD_PTR},
};

#[repr(C)]
pub struct SerializationRoots {
    pub actor: Value,
    pub dedup_table: Value,
    pub migrations_list: Value,
}

use self::stable_memory_stream::{ScanStream, StableMemoryStream};

use super::{
    graph_copy::{limit::ExecutionMonitor, GraphCopy},
    layout::{scan_serialized, StableToSpace, StableValue},
    DUMMY_VALUE,
};

pub struct Serialization {
    to_space: StableMemoryStream,
    limit: ExecutionMonitor,
    array_slice: Option<ArraySlice>,
    pub dedup_table_address: StableValue,
    pub migrations_list_address: StableValue,
}

pub struct ArraySlice {
    pub next_index: u64,
    pub array_length: u64,
}

impl ArraySlice {
    pub fn new(index: u64, length: u64) -> ArraySlice {
        ArraySlice {
            next_index: index,
            array_length: length,
        }
    }
}

/// Helper type to pass serialization context instead of closures.
pub struct SerializationContext<'a, M> {
    pub serialization: &'a mut Serialization,
    pub mem: &'a mut M,
}

impl<'a, M> SerializationContext<'a, M> {
    fn new(serialization: &'a mut Serialization, mem: &'a mut M) -> SerializationContext<'a, M> {
        SerializationContext { serialization, mem }
    }
}

/// Graph-copy-based serialization.
/// Notes:
/// - Invalidates the heap by replacing reachable stable object by forwarding objects:
/// The heap is finally no longer usable by mutator or GC.
/// - `copy` and partially also `scan` depends on the heap layout. Adjust these functions
/// whenever the heap layout is changed.
/// Usage:
/// ```
/// let serialization = Serialization::start(root, stable_start);
/// while !serialization.is_completed() {
///     serialization.copy_increment();
/// }
/// ```
impl Serialization {
    /// Start the graph-copy-based heap serialization from the stable `root` object
    /// by writing the serialized data to the stable memory at offset `stable_start`.
    /// The start is followed by a series of copy increments before the serialization is completed.
    pub fn start<M: Memory>(
        mem: &mut M,
        roots: SerializationRoots,
        stable_start: u64,
    ) -> Serialization {
        let to_space = StableMemoryStream::open(stable_start);
        let limit = ExecutionMonitor::new();
        let mut serialization = Serialization {
            limit,
            to_space,
            array_slice: None,
            // Dummy start values. Will be overwritten later via serialization.start().
            dedup_table_address: StableValue::from_raw(0),
            migrations_list_address: StableValue::from_raw(0),
        };
        // Start serializing from all three roots: the actor and the GC helper roots.
        let _ = serialization.start(mem, roots.actor);
        if roots.dedup_table.is_non_null_ptr() {
            serialization.dedup_table_address = serialization.start(mem, roots.dedup_table);
        }
        if roots.migrations_list.is_non_null_ptr() {
            serialization.migrations_list_address = serialization.start(mem, roots.migrations_list);
        }
        serialization
    }

    pub fn serialized_data_start(&self) -> u64 {
        self.to_space.base_address()
    }

    pub fn serialized_data_length(&self) -> u64 {
        self.to_space.written_length()
    }

    /// Resolve the Brooks forwarding pointer of the incremental GC by considering potential
    /// forwarding objects (`FwdPtr`) used in Cheney's algorithm for stabilization.
    unsafe fn resolve_gc_forwarding(object: Value) -> Value {
        let tag = Self::read_object_tag(object);
        if tag == TAG_FWD_PTR {
            object
        } else {
            object.forward()
        }
    }
    /// Read the object tag by considering potential forwarding objects (`FwdPtr`).
    unsafe fn read_object_tag(object: Value) -> Tag {
        // Do not call `tag()` as it dereferences the Brooks forwarding pointer of the incremental GC,
        // which does not exist for the forwarding objects (`FwdPtr`) used by the Cheney's algorithm.
        *(object.get_ptr() as *const Tag)
    }

    fn has_non_stable_type(old_field: Value) -> bool {
        unsafe { old_field.tag() == TAG_CLOSURE }
    }

    pub fn pending_array_scanning(&self) -> bool {
        self.array_slice.is_some()
    }

    pub fn get_array_slice(&mut self) -> ArraySlice {
        self.array_slice.take().unwrap()
    }

    pub fn set_array_slice(&mut self, slice: ArraySlice) {
        self.array_slice = Some(slice);
    }

    fn processed_memory(&self) -> u64 {
        self.to_space.written_length() + self.to_space.scanned_length()
    }
}

impl GraphCopy<Value, StableValue, u32> for Serialization {
    fn get_forward_address(&self, object: Value) -> Option<StableValue> {
        unsafe {
            let object = Self::resolve_gc_forwarding(object);
            let tag = Self::read_object_tag(object);
            match tag {
                TAG_FWD_PTR => {
                    let new_location = (*(object.get_ptr() as *mut FwdPtr)).fwd;
                    Some(StableValue::serialize(new_location))
                }
                _ => None,
            }
        }
    }

    fn set_forward_address(&mut self, object: Value, target: StableValue) {
        unsafe {
            let object = Self::resolve_gc_forwarding(object);
            debug_assert!(object.is_obj());
            let fwd = object.get_ptr() as *mut FwdPtr;
            (*fwd).tag = TAG_FWD_PTR;
            (*fwd).fwd = target.deserialize();
        }
    }

    fn copy<M: Memory>(&mut self, _mem: &mut M, object: Value) -> StableValue {
        unsafe {
            let object = Self::resolve_gc_forwarding(object);
            debug_assert!(object.is_obj());
            let address = self.to_space.written_length();
            serialize(&mut self.to_space, object);
            debug_assert!(self.to_space.written_length() >= address);
            StableValue::from_stable_address(address)
        }
    }

    fn scan<M: Memory>(&mut self, mem: &mut M) {
        scan_serialized(
            &mut SerializationContext::new(self, mem),
            &|context, original| {
                let old_value = original.deserialize();
                if old_value.is_non_null_ptr() {
                    if Self::has_non_stable_type(old_value) {
                        // Due to structural subtyping or `Any`-subtyping, a non-stable object (such as a closure) may be
                        // be dynamically reachable from a stable varibale. The value is not accessible in the new program version.
                        // Therefore, the content of these fields can serialized with a dummy value that is also ignored by the GC.
                        DUMMY_VALUE
                    } else {
                        context.serialization.evacuate(context.mem, old_value)
                    }
                } else {
                    original
                }
            },
        );
    }

    fn scanning_completed(&self) -> bool {
        self.to_space.scan_completed()
    }

    fn complete(&mut self) {
        self.to_space.close();
    }

    fn time_over(&mut self) -> bool {
        self.limit.is_exceeded(self.processed_memory())
    }

    fn reset_time(&mut self) {
        self.limit.reset(self.processed_memory());
    }
}

impl StableToSpace for Serialization {
    fn to_space(&mut self) -> &mut StableMemoryStream {
        &mut self.to_space
    }
}

pub mod metadata;
mod performance;

use motoko_rts_macros::ic_mem_fn;

use crate::{
    gc::incremental::{is_gc_stopped, resume_gc, stop_gc},
    memory::Memory,
    persistence::{
        compatibility::TypeDescriptor, get_dedup_table_ptr, get_migration_functions_ptr,
        restore_stable_type, set_dedup_table_ptr, set_migration_functions_ptr,
        set_upgrade_instructions,
    },
    rts_trap_with,
    stabilization::ic::metadata::StabilizationMetadata,
    stabilization::serialization::SerializationRoots,
    stable_mem::{self, moc_stable_mem_set_size, PAGE_SIZE},
    types::Value,
};

use self::{metadata::UpgradeStatistics, performance::InstructionMeter};

use super::graph_copy::GraphCopy;
use super::{deserialization::Deserialization, serialization::Serialization};

struct StabilizationState {
    old_candid_data: Value,
    old_type_offsets: Value,
    completed: bool,
    pub serialization: Serialization,
    pub instruction_meter: InstructionMeter,
}

impl StabilizationState {
    fn new(
        serialization: Serialization,
        old_candid_data: Value,
        old_type_offsets: Value,
    ) -> StabilizationState {
        StabilizationState {
            old_candid_data,
            old_type_offsets,
            completed: false,
            serialization,
            instruction_meter: InstructionMeter::new(),
        }
    }
}

static mut STABILIZATION_STATE: Option<StabilizationState> = None;

#[no_mangle]
pub unsafe extern "C" fn is_graph_stabilization_started() -> bool {
    STABILIZATION_STATE.is_some()
}

/// Start the incremental graph-copy-based stabilization.
/// This operations initiates the graph copy stabilization instead of enhanced orthogonal persistence.
/// It is required before a series of stabilization increments can be run.
/// `stable_actor`: Root object for stabilization containing all stable variables of the actor.
/// `old_candid_data`: A blob encoding the Candid type as a table.
/// `old_type_offsets`: A blob encoding the type offsets in the Candid type table.
///   Type index 0 represents the stable actor object to be serialized.
/// Note:
/// - Once started, the heap is invalidated. All application messages must be blocked after this start.
#[ic_mem_fn(ic_only)]
pub unsafe fn start_graph_stabilization<M: Memory>(
    mem: &mut M,
    stable_actor: Value,
    old_candid_data: Value,
    old_type_offsets: Value,
) {
    assert!(STABILIZATION_STATE.is_none());
    assert!(is_gc_stopped());
    let stable_memory_pages = stable_mem::size(); // Backup the virtual size.
    let serialized_data_start = stable_memory_pages * PAGE_SIZE;
    let serialization_roots = SerializationRoots {
        actor: stable_actor,
        dedup_table: *get_dedup_table_ptr(),
        migrations_list: *get_migration_functions_ptr(),
    };
    let serialization = Serialization::start(mem, serialization_roots, serialized_data_start);
    STABILIZATION_STATE = Some(StabilizationState::new(
        serialization,
        old_candid_data,
        old_type_offsets,
    ));
}

/// Incremental graph-copy-based stabilization, serializing a limited amount of heap objects reachable
/// from stable variables into stable memory.
/// This function can be called multiple times before the upgrade of a large heap.
/// The incrementality serves to support the graph-copy-based serialization large heaps that do not fit
/// into the upgrade message instruction limit.
/// Returns true if the stabilization has been completed.
/// Notes:
/// - During stabilization, the heap is invalidated. Therefore and for consistent serialization,
///   all application messages must be blocked during this process.
/// - This operation only runs a limited number of instructions that may not yet complete the stabilization.
///   The compiler may needs to trigger more messages that run additional stabilzation increments, before the
///   stabilization has been completed and the actual canister upgrade is ready to be performed.
/// Implementation:
/// * Algorithm: Cheney's algorithm using main memory as from-space and stable memory as to-space.
/// * Encoding: The from-space uses the main memory heap layout, while the to-space is encoded in
///   the stable object graph layout (see `GraphCopyStabilization.md`).
#[ic_mem_fn(ic_only)]
pub unsafe fn graph_stabilization_increment<M: Memory>(mem: &mut M) -> bool {
    let state = STABILIZATION_STATE.as_mut().unwrap();
    if !state.completed {
        assert!(is_gc_stopped());
        state.instruction_meter.start();
        state.serialization.copy_increment(mem);
        state.instruction_meter.stop();
        if state.serialization.is_completed() {
            write_metadata();
            state.completed = true;
        }
    }
    state.completed
}

unsafe fn write_metadata() {
    let state = STABILIZATION_STATE.as_mut().unwrap();
    assert!(state.serialization.is_completed());
    state.instruction_meter.start();
    let serialized_data_start = state.serialization.serialized_data_start();
    let serialized_data_length = state.serialization.serialized_data_length();

    let type_descriptor = TypeDescriptor::new(state.old_candid_data, state.old_type_offsets);
    let metadata = StabilizationMetadata {
        serialized_data_start,
        serialized_data_length,
        type_descriptor,
    };
    state.instruction_meter.stop();
    metadata.store(
        &mut state.instruction_meter,
        state.serialization.dedup_table_address,
        state.serialization.migrations_list_address,
    );
}

struct DestabilizationState {
    deserialization: Deserialization,
    stabilization_statistics: UpgradeStatistics,
    completed: bool,
    instruction_meter: InstructionMeter,
}

static mut DESTABILIZATION_STATE: Option<DestabilizationState> = None;

/// Starts the graph-copy-based destabilization process.
/// This requires that the deserialization is subsequently run and completed.
/// The old type descriptor from stable memory is restored into `PersistentMetadata`
/// so that `register_stable_type` can check compatibility when the migration chain
/// runs after destabilization (inside `Persistence.load` / `ICStableRead`).
#[ic_mem_fn(ic_only)]
pub unsafe fn start_graph_destabilization<M: Memory>(mem: &mut M) {
    assert!(DESTABILIZATION_STATE.is_none());

    let mut instruction_meter = InstructionMeter::new();
    instruction_meter.start();
    let (metadata, last_page_record) = StabilizationMetadata::load(mem);
    restore_stable_type(mem, &metadata.type_descriptor);
    moc_stable_mem_set_size(metadata.serialized_data_start / PAGE_SIZE);

    // Stop the GC until the incremental graph destabilization has been completed.
    stop_gc();

    let deserialization = Deserialization::start(
        mem,
        metadata.serialized_data_start,
        metadata.serialized_data_length,
        last_page_record.dedup_table_address,
        last_page_record.migrations_list_address,
    );
    instruction_meter.stop();
    DESTABILIZATION_STATE = Some(DestabilizationState {
        deserialization,
        stabilization_statistics: last_page_record.statistics,
        completed: false,
        instruction_meter,
    });
}

/// Incremental graph-copy-based destabilization, deserializing a limited amount of serialized data from
/// stable memory to the heap.
/// This function can be called multiple times after the upgrade of a large heap.
/// The incrementality serves to support the graph-copy-based deserialization of large heaps that do not fit
/// into the upgrade message instruction limit.
/// Returns true if the destabilization has been completed.
/// Notes:
/// - The heap is only valid after completed destabilization. Therefore and for consistent deserialization,
///   all application messages must be blocked until this is completed.
/// - This operation only runs a limited number of instructions that may not yet complete the upgrade.
///   The compiler may need to trigger more messages that run additional destabilzation increments, before
///   the destabilization is completed and the application code can resume its operation.
/// Implementation:
/// * Algorithm: Cheney's algorithm using stable memory as from-space and main memory as to-space.
/// * Encoding: The from-space uses the stable memory layout, while the to-space is to be encoded in
///   main memory layout (see `GraphCopyStabilization.md`).
#[ic_mem_fn(ic_only)]
pub unsafe fn graph_destabilization_increment<M: Memory>(mem: &mut M) -> bool {
    let state = DESTABILIZATION_STATE
        .as_mut()
        .unwrap_or_else(|| rts_trap_with("No destabilization needed"));
    if !state.completed {
        assert!(is_gc_stopped());
        state.instruction_meter.start();
        state.deserialization.copy_increment(mem);
        state.instruction_meter.stop();
        if state.deserialization.is_completed() {
            record_upgrade_costs();

            // We need to put back in the metadata pointing to the
            // helper GC roots for the dedup table and migration list.
            set_dedup_table_ptr(mem, state.deserialization.dedup_table_address);
            set_migration_functions_ptr(mem, state.deserialization.migrations_list_address);

            state.completed = true;
            memory_sanity_check(mem);
        }
    }
    state.completed
}

unsafe fn memory_sanity_check<M: Memory>(_mem: &mut M) {
    #[cfg(feature = "memory_check")]
    {
        use crate::gc::incremental::{
            get_partitioned_heap,
            sanity_checks::{check_memory, CheckerMode},
        };

        let state = DESTABILIZATION_STATE.as_mut().unwrap();
        let unused_root = &mut Value::from_scalar(0) as *mut Value;
        let roots = [
            &mut state.deserialization.get_stable_root() as *mut Value,
            unused_root,
            unused_root,
            unused_root,
            unused_root,
            unused_root,
        ];
        check_memory(
            _mem,
            get_partitioned_heap(),
            roots,
            CheckerMode::UpdateCompletion,
        );
    }
}

unsafe fn record_upgrade_costs() {
    let state = DESTABILIZATION_STATE.as_ref().unwrap();
    let total_instructions = state.stabilization_statistics.stabilization_instructions
        + state.instruction_meter.total_elapsed();
    set_upgrade_instructions(total_instructions);
}

/// Returns the deserialized stable actor root after the completed destabilization.
#[no_mangle]
pub unsafe extern "C" fn get_graph_destabilized_actor() -> Value {
    let state = DESTABILIZATION_STATE.as_ref().unwrap();
    assert!(state.completed);
    state.deserialization.get_stable_root()
}

/// Stop the GC before performing incremental graph-copy-based stabilzation or destabilization.
/// This is only a safe-guard since the compiler must not schedule the GC during stabilization
/// and destabilization.
#[no_mangle]
pub unsafe extern "C" fn stop_gc_before_stabilization() {
    stop_gc();
}

/// Start the GC after completed incremental graph-copy-based destabilization.
#[no_mangle]
pub unsafe extern "C" fn start_gc_after_destabilization() {
    resume_gc();
}

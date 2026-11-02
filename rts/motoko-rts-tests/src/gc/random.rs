use super::TestHeap;
use super::utils::ObjectIdx;

use oorandom::Rand32;

fn rand_bool(rng: &mut Rand32) -> bool {
    rng.rand_range(0..2) == 1
}

pub fn generate(seed: u64, max_objects: usize) -> TestHeap {
    let mut rng = Rand32::new(seed);

    let n_objects = rng.rand_range(0..max_objects as u32 + 1);

    let roots: Vec<ObjectIdx> = (0..n_objects)
        .filter_map(|obj_idx| {
            if rand_bool(&mut rng) {
                Some(obj_idx as usize)
            } else {
                None
            }
        })
        .collect();

    let heap: Vec<(ObjectIdx, Vec<ObjectIdx>)> = (0..n_objects as usize)
        .map(|obj_idx| {
            let n_fields = rng.rand_range(0..n_objects) as usize;

            let field_values = (0..n_fields)
                .filter_map(|_field_idx| {
                    let field_value = rng.rand_range(0..n_objects) as usize;
                    if field_value == obj_idx {
                        None
                    } else {
                        Some(field_value)
                    }
                })
                .collect::<Vec<ObjectIdx>>();

            (obj_idx, field_values)
        })
        .collect();

    // Same as roots
    let continuation_table: Vec<ObjectIdx> = (0..n_objects as usize)
        .filter_map(|obj_idx| {
            if rand_bool(&mut rng) {
                Some(obj_idx)
            } else {
                None
            }
        })
        .collect();

    TestHeap {
        heap,
        roots,
        continuation_table,
    }
}

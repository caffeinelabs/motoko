pub mod enhanced;

use crate::types::Value;

pub type Roots = self::enhanced::Roots;

#[cfg(feature = "ic")]
pub unsafe fn root_set() -> Roots {
    self::enhanced::root_set()
}

pub unsafe fn visit_roots<C, V: Fn(&mut C, *mut Value)>(
    roots: Roots,
    _heap_base: usize,
    context: &mut C,
    visit_field: V,
) {
    self::enhanced::visit_roots(roots, context, visit_field);
}

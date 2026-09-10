#[enhanced_orthogonal_persistence]
pub mod enhanced;

use motoko_rts_macros::enhanced_orthogonal_persistence;

use crate::types::Value;

#[enhanced_orthogonal_persistence]
pub type Roots = self::enhanced::Roots;

#[cfg(feature = "ic")]
#[enhanced_orthogonal_persistence]
pub unsafe fn root_set() -> Roots {
    self::enhanced::root_set()
}

#[enhanced_orthogonal_persistence]
pub unsafe fn visit_roots<C, V: Fn(&mut C, *mut Value)>(
    roots: Roots,
    _heap_base: usize,
    context: &mut C,
    visit_field: V,
) {
    self::enhanced::visit_roots(roots, context, visit_field);
}

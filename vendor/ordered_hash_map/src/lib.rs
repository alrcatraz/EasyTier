#![cfg_attr(not(any(test, doc)), no_std)]
#![allow(dead_code)]
#![warn(
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::from_iter_instead_of_collect,
    clippy::indexing_slicing,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mut_mut,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    explicit_outlives_requirements,
    keyword_idents,
    let_underscore_drop,
    macro_use_extern_crate,
    missing_debug_implementations,
    non_ascii_idents,
    noop_method_call,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_op_in_unsafe_fn,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_macro_rules,
    unused_qualifications
)]
#![deny(missing_docs)]

//! This crate offers an ordered hash map and its corresponding set. These structures have the
//! performance characteristics of a HashMap/Set for random access but where insertion order is
//! preserved and accessible. Both expose an API similar to the std collections but all iterators
//! operate in insertion order.

mod map;
/// An Ordered Map implementation
pub mod ordered_map {
    pub use crate::map::*;
}
pub use map::OrderedHashMap;

mod set;
/// An Ordered Set implementation
pub mod ordered_set {
    pub use crate::set::*;
}
pub use set::OrderedHashSet;

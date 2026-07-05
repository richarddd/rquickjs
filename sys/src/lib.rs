#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::uninlined_format_args)]
#![no_std]

use ::core::ptr;

/// Common error message for converting between C `size_t` and Rust `usize`;
pub const SIZE_T_ERROR: &str =
    "conversion between C type 'size_t' and Rust type 'usize' overflowed.";

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// The two QuickJS flavors are mutually exclusive because the engines have
// divergent ABIs and APIs. Enabling both is an error. Enabling neither falls
// back to quickjs-ng: this keeps flavor selection ergonomic (quickjs-ng is the
// implicit default) and lets tooling that builds against `rquickjs` with
// `default-features = false` — e.g. trybuild's generated crates — still resolve
// a flavor without every consumer having to re-declare one.
#[cfg(all(feature = "quickjs-ng", feature = "quickjs-og"))]
compile_error!("features `quickjs-ng` and `quickjs-og` are mutually exclusive; enable only one");

// Pre-generated (bundled) bindings are only shipped for the quickjs-ng flavor.
// When building against the original QuickJS the `bindgen` feature is required
// so the bindings match the selected C sources.
#[cfg(all(feature = "quickjs-og", not(feature = "bindgen")))]
compile_error!(
    "the `quickjs-og` flavor requires the `bindgen` feature; \
     bundled bindings are only provided for `quickjs-ng`"
);

#[cfg(all(not(feature = "bindgen"), not(feature = "quickjs-og")))]
include!(concat!("bindings/", bindings_env!("TARGET"), ".rs"));

// The flavor-specific inline helpers are gated so that when both flavors are
// (invalidly) enabled, neither set is pulled in and the `compile_error!` above
// is the only diagnostic the user sees, rather than a wall of duplicate-symbol
// errors.
#[cfg(all(
    target_pointer_width = "64",
    not(all(feature = "quickjs-og", not(feature = "quickjs-ng")))
))]
include!("inlines/ptr_64.rs");

#[cfg(all(
    target_pointer_width = "64",
    feature = "quickjs-og",
    not(feature = "quickjs-ng")
))]
include!("inlines/ptr_64_original.rs");

#[cfg(target_pointer_width = "32")]
include!("inlines/ptr_32_nan_boxing.rs");

include!("inlines/common.rs");

#[cfg(all(feature = "quickjs-og", not(feature = "quickjs-ng")))]
include!("inlines/compat_original.rs");

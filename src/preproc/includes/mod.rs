mod diagnostics;
mod directives;
mod expr;
#[cfg(not(target_arch = "wasm32"))]
mod paths;
#[cfg(not(target_arch = "wasm32"))]
mod resolution;
#[cfg(not(target_arch = "wasm32"))]
mod stdlib;
mod target;
mod url;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub(super) use directives::{
    consume_preprocessor_block, evaluate_assert_expression, find_matching_endfor,
    find_matching_endwhile, parse_preprocess_directive,
};
pub(super) use expr::{
    eval_int_expr, eval_simple_arithmetic, evaluate_preprocess_expr, evaluate_scalar_expr,
};
#[cfg(not(target_arch = "wasm32"))]
pub(super) use resolution::{
    process_import_directive, process_include_directive, process_include_many_directive,
    ImportDirectiveContext,
};
pub(super) use url::{extract_url, fetch_url_include};
#[cfg(target_arch = "wasm32")]
pub(super) use wasm::{
    include_not_supported_in_wasm, process_import_directive, process_include_directive,
    process_include_many_directive,
};

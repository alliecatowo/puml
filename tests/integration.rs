pub(crate) use assert_cmd::Command;
pub(crate) use image::GenericImageView;
pub(crate) use predicates::prelude::*;
pub(crate) use puml::model::SequenceEventKind;
pub(crate) use puml::normalize;
pub(crate) use puml::parser::parse;
pub(crate) use puml::{render_source_to_svg, render_source_to_text, TextOutputMode};
pub(crate) use serde_json::Value;
pub(crate) use std::fs;
pub(crate) use tempfile::tempdir;

pub(crate) fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn example(name: &str) -> String {
    format!("{}/docs/examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

macro_rules! assert_snapshot {
    ($name:expr, $value:expr $(,)?) => {{
        let snapshot_name = format!("integration__{}", $name);
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            snapshot_path => "../snapshots",
        }, {
            insta::assert_snapshot!(snapshot_name, $value);
        })
    }};
    ($name:expr, $value:expr, $debug_expr:expr $(,)?) => {{
        let snapshot_name = format!("integration__{}", $name);
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            snapshot_path => "../snapshots",
        }, {
            insta::assert_snapshot!(snapshot_name, $value, $debug_expr);
        })
    }};
    ($value:expr $(,)?) => {
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            snapshot_path => "../snapshots",
        }, {
            insta::assert_snapshot!($value);
        })
    };
}

macro_rules! assert_json_snapshot {
    ($name:expr, $value:expr $(,)?) => {{
        let snapshot_name = format!("integration__{}", $name);
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            snapshot_path => "../snapshots",
        }, {
            insta::assert_json_snapshot!(snapshot_name, $value);
        })
    }};
    ($name:expr, $value:expr, $debug_expr:expr $(,)?) => {{
        let snapshot_name = format!("integration__{}", $name);
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            snapshot_path => "../snapshots",
        }, {
            insta::assert_json_snapshot!(snapshot_name, $value, $debug_expr);
        })
    }};
    ($value:expr $(,)?) => {
        insta::with_settings!({
            prepend_module_to_snapshot => false,
            snapshot_path => "../snapshots",
        }, {
            insta::assert_json_snapshot!($value);
        })
    };
}

#[path = "integration/diagnostics.rs"]
mod diagnostics;
#[path = "integration/dialects.rs"]
mod dialects;
#[path = "integration/dump_multi.rs"]
mod dump_multi;
#[path = "integration/families_models.rs"]
mod families_models;
#[path = "integration/family_rendering_a.rs"]
mod family_rendering_a;
#[path = "integration/family_rendering_b.rs"]
mod family_rendering_b;
#[path = "integration/fixture_families.rs"]
mod fixture_families;
#[path = "integration/includes.rs"]
mod includes;
#[path = "integration/lint_cli.rs"]
mod lint_cli;
#[path = "integration/markdown.rs"]
mod markdown;
#[path = "integration/mindmap_layout.rs"]
mod mindmap_layout;
#[path = "integration/nonuml_advanced.rs"]
mod nonuml_advanced;
#[path = "integration/output_formats.rs"]
mod output_formats;
#[path = "integration/preproc_more.rs"]
mod preproc_more;
#[path = "integration/preprocessor.rs"]
mod preprocessor;
#[path = "integration/preprocessor_core.rs"]
mod preprocessor_core;
#[path = "integration/salt_advanced.rs"]
mod salt_advanced;
#[path = "integration/skinparams_theme.rs"]
mod skinparams_theme;
#[path = "integration/state_and_creole.rs"]
mod state_and_creole;
#[path = "integration/stdlib.rs"]
mod stdlib;
#[path = "integration/structure_io.rs"]
mod structure_io;
#[path = "integration/support.rs"]
mod support;
#[path = "integration/validation_corpus.rs"]
mod validation_corpus;

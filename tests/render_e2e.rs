macro_rules! assert_snapshot {
    ($name:expr, $value:expr $(,)?) => {{
        let snapshot_value = insta::_macro_support::format!("{}", &$value);
        insta::_macro_support::assert_snapshot(
            ($name, snapshot_value.as_str()).into(),
            insta::_get_workspace_root!().as_path(),
            insta::_function_name!(),
            "render_e2e",
            "tests/render_e2e.rs",
            line!(),
            stringify!($value),
        )
        .unwrap()
    }};
}

#[path = "render_e2e/core.rs"]
mod core;
#[path = "render_e2e/overflow.rs"]
mod overflow;
#[path = "render_e2e/sequence_features.rs"]
mod sequence_features;
#[path = "render_e2e/sequence_layout.rs"]
mod sequence_layout;
#[path = "render_e2e/support.rs"]
mod support;

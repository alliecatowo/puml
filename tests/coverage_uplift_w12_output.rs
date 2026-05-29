//! Wave-12 coverage uplift — output module focused tests.
//!
//! Targets `src/output/svg_postprocess.rs` and `src/output/contract.rs`
//! which were measured at ~76–82% line coverage in the wave-12 baseline.
//! Each test asserts specific documented behaviour of the output pipeline.
//!
//! Refs #89

use puml::model::ScaleSpec;
use puml::output::{
    append_mainframe_svg, append_optional_mainframe_svg, apply_scale_svg, CommonCommandKind,
    CommonCommandPath, RenderArtifact, RenderCommonCommands, RenderSceneContract,
};
use puml::render_core::SceneAvailability;

// ── svg_postprocess.rs: apply_scale_svg ───────────────────────────────────────

fn svg_with_dims(w: u32, h: u32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"></svg>"
    )
}

#[test]
fn scale_factor_updates_both_dimensions() {
    let mut svg = svg_with_dims(100, 50);
    apply_scale_svg(&mut svg, &ScaleSpec::Factor(2.0));
    assert!(svg.contains("width=\"200\""), "width should double");
    assert!(svg.contains("height=\"100\""), "height should double");
    assert!(svg.contains("viewBox=\"0 0 100 50\""), "viewBox unchanged");
}

#[test]
fn scale_factor_half_rounds_correctly() {
    let mut svg = svg_with_dims(100, 80);
    apply_scale_svg(&mut svg, &ScaleSpec::Factor(0.5));
    assert!(svg.contains("width=\"50\""));
    assert!(svg.contains("height=\"40\""));
}

#[test]
fn scale_width_preserves_aspect_ratio() {
    let mut svg = svg_with_dims(200, 100);
    apply_scale_svg(&mut svg, &ScaleSpec::Width(400));
    assert!(svg.contains("width=\"400\""));
    assert!(svg.contains("height=\"200\""));
}

#[test]
fn scale_height_preserves_aspect_ratio() {
    let mut svg = svg_with_dims(200, 100);
    apply_scale_svg(&mut svg, &ScaleSpec::Height(200));
    assert!(svg.contains("height=\"200\""));
    assert!(svg.contains("width=\"400\""));
}

#[test]
fn scale_fixed_sets_exact_dimensions() {
    let mut svg = svg_with_dims(100, 50);
    apply_scale_svg(
        &mut svg,
        &ScaleSpec::Fixed {
            width: 300,
            height: 150,
        },
    );
    assert!(svg.contains("width=\"300\""));
    assert!(svg.contains("height=\"150\""));
}

#[test]
fn scale_max_does_not_scale_up_smaller_image() {
    let mut svg = svg_with_dims(50, 30);
    apply_scale_svg(&mut svg, &ScaleSpec::Max(200));
    // 50x30 is already below the max — should stay unchanged
    assert!(svg.contains("width=\"50\""));
    assert!(svg.contains("height=\"30\""));
}

#[test]
fn scale_max_scales_down_larger_image() {
    let mut svg = svg_with_dims(400, 200);
    apply_scale_svg(&mut svg, &ScaleSpec::Max(100));
    // Larger dimension is 400, factor = 100/400 = 0.25
    assert!(svg.contains("width=\"100\""));
    assert!(svg.contains("height=\"50\""));
}

#[test]
fn scale_max_width_does_not_scale_up() {
    let mut svg = svg_with_dims(80, 40);
    apply_scale_svg(&mut svg, &ScaleSpec::MaxWidth(200));
    assert!(svg.contains("width=\"80\""));
    assert!(svg.contains("height=\"40\""));
}

#[test]
fn scale_max_width_scales_down_when_too_wide() {
    let mut svg = svg_with_dims(400, 100);
    apply_scale_svg(&mut svg, &ScaleSpec::MaxWidth(200));
    assert!(svg.contains("width=\"200\""));
    assert!(svg.contains("height=\"50\""));
}

#[test]
fn scale_max_height_does_not_scale_up() {
    let mut svg = svg_with_dims(80, 40);
    apply_scale_svg(&mut svg, &ScaleSpec::MaxHeight(200));
    assert!(svg.contains("width=\"80\""));
    assert!(svg.contains("height=\"40\""));
}

#[test]
fn scale_max_height_scales_down_when_too_tall() {
    let mut svg = svg_with_dims(100, 400);
    apply_scale_svg(&mut svg, &ScaleSpec::MaxHeight(200));
    assert!(svg.contains("height=\"200\""));
    assert!(svg.contains("width=\"50\""));
}

#[test]
fn scale_max_fixed_both_fit_no_change() {
    let mut svg = svg_with_dims(50, 30);
    apply_scale_svg(
        &mut svg,
        &ScaleSpec::MaxFixed {
            width: 200,
            height: 200,
        },
    );
    assert!(svg.contains("width=\"50\""));
    assert!(svg.contains("height=\"30\""));
}

#[test]
fn scale_max_fixed_width_constrained() {
    let mut svg = svg_with_dims(400, 100);
    apply_scale_svg(
        &mut svg,
        &ScaleSpec::MaxFixed {
            width: 200,
            height: 300,
        },
    );
    // Width is the binding constraint: factor = 200/400 = 0.5
    assert!(svg.contains("width=\"200\""));
    assert!(svg.contains("height=\"50\""));
}

#[test]
fn scale_max_fixed_height_constrained() {
    let mut svg = svg_with_dims(100, 400);
    apply_scale_svg(
        &mut svg,
        &ScaleSpec::MaxFixed {
            width: 300,
            height: 200,
        },
    );
    // Height is the binding constraint: factor = 200/400 = 0.5
    assert!(svg.contains("width=\"50\""));
    assert!(svg.contains("height=\"200\""));
}

#[test]
fn scale_noop_when_svg_has_no_dimensions() {
    let mut svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_string();
    let original = svg.clone();
    apply_scale_svg(&mut svg, &ScaleSpec::Factor(2.0));
    // No dimensions to parse, so no change expected
    assert_eq!(svg, original);
}

#[test]
fn scale_noop_when_svg_has_zero_dimensions() {
    let mut svg =
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"></svg>".to_string();
    let original = svg.clone();
    apply_scale_svg(&mut svg, &ScaleSpec::Factor(2.0));
    // 0 dimensions are treated as noop
    assert_eq!(svg, original);
}

// ── svg_postprocess.rs: append_mainframe_svg ─────────────────────────────────

#[test]
fn mainframe_is_inserted_before_closing_svg_tag() {
    let mut svg = svg_with_dims(200, 100);
    append_mainframe_svg(&mut svg, "Main");
    assert!(svg.contains("class=\"uml-mainframe\""));
    assert!(svg.contains("class=\"uml-mainframe-title\""));
    assert!(svg.ends_with("</svg>"));
}

#[test]
fn mainframe_empty_title_renders_frame_without_text() {
    let mut svg = svg_with_dims(200, 100);
    append_mainframe_svg(&mut svg, "");
    assert!(svg.contains("class=\"uml-mainframe\""));
    // Empty title → no text element for the title label
    assert!(!svg.contains("<text"));
}

#[test]
fn mainframe_noop_when_svg_too_small() {
    // Width <= 8 or height <= 8 → mainframe is skipped
    let mut svg_small =
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"></svg>".to_string();
    let original = svg_small.clone();
    append_mainframe_svg(&mut svg_small, "Title");
    assert_eq!(
        svg_small, original,
        "mainframe should be skipped for tiny SVG"
    );
}

#[test]
fn mainframe_noop_when_svg_has_no_dimensions() {
    let mut svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_string();
    let original = svg.clone();
    append_mainframe_svg(&mut svg, "Title");
    assert_eq!(svg, original);
}

#[test]
fn mainframe_noop_when_svg_has_no_closing_tag() {
    let mut svg =
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\">".to_string();
    let original = svg.clone();
    append_mainframe_svg(&mut svg, "Title");
    assert_eq!(svg, original);
}

#[test]
fn optional_mainframe_skips_none() {
    let mut svg = svg_with_dims(200, 100);
    let original = svg.clone();
    append_optional_mainframe_svg(&mut svg, None);
    assert_eq!(svg, original);
}

#[test]
fn optional_mainframe_applies_some() {
    let mut svg = svg_with_dims(200, 100);
    append_optional_mainframe_svg(&mut svg, Some("Frame Title"));
    assert!(svg.contains("class=\"uml-mainframe\""));
}

#[test]
fn mainframe_title_with_long_name_clamps_notch_width() {
    let long_title = "A".repeat(200);
    let mut svg = svg_with_dims(300, 100);
    append_mainframe_svg(&mut svg, &long_title);
    // Should not panic; the notch width is clamped to width - 2 * INSET
    assert!(svg.contains("class=\"uml-mainframe\""));
}

// ── contract.rs: RenderCommonCommands ─────────────────────────────────────────

#[test]
fn render_common_commands_default_is_empty() {
    let cmds = RenderCommonCommands::default();
    assert!(cmds.is_empty());
    assert!(cmds.scale.is_none());
    assert!(cmds.mainframe.is_none());
    assert!(cmds.applications.is_empty());
}

#[test]
fn render_common_commands_from_parts() {
    let cmds = RenderCommonCommands::from_parts(Some(ScaleSpec::Factor(2.0)), None);
    assert!(!cmds.is_empty());
    assert!(cmds.scale.is_some());
    assert!(cmds.mainframe.is_none());
}

#[test]
fn render_common_commands_with_mainframe_not_empty() {
    let cmds = RenderCommonCommands::from_parts(None, Some("Title".to_string()));
    assert!(!cmds.is_empty());
}

// ── contract.rs: RenderArtifact ───────────────────────────────────────────────

#[test]
fn render_artifact_default_has_empty_svg() {
    let artifact = RenderArtifact::default();
    assert!(artifact.svg.is_empty());
    assert!(artifact.diagnostics.is_empty());
    assert!(artifact.scene.is_none());
}

#[test]
fn render_artifact_svg_only_constructor_parses_dimensions() {
    let svg = svg_with_dims(200, 100);
    let artifact = RenderArtifact::svg_only(svg);
    let dims = artifact.dimensions.expect("dimensions should be set");
    assert_eq!(dims.width, 200.0);
    assert_eq!(dims.height, 100.0);
}

#[test]
fn render_artifact_svg_only_no_scene() {
    let artifact = RenderArtifact::svg_only(svg_with_dims(100, 50));
    assert!(artifact.typed_scene().is_none());
    assert!(
        artifact.require_typed_scene().is_err(),
        "should be an error for not-migrated artifact"
    );
}

#[test]
fn render_artifact_unsupported_has_correct_availability() {
    let artifact = RenderArtifact::unsupported(svg_with_dims(100, 50));
    assert!(matches!(
        artifact.scene_contract(),
        RenderSceneContract::Unsupported
    ));
    assert!(artifact.require_typed_scene().is_err());
}

#[test]
fn render_artifact_with_scene_availability_typed_but_no_scene_falls_back() {
    // If TypedScene is forced but scene is None, it should downgrade to NotMigrated
    let artifact = RenderArtifact::svg_only(svg_with_dims(100, 50))
        .with_scene_availability(SceneAvailability::TypedScene);
    assert!(matches!(
        artifact.scene_contract(),
        RenderSceneContract::NotMigrated
    ));
}

#[test]
fn render_artifact_with_scene_availability_unsupported() {
    let artifact = RenderArtifact::svg_only(svg_with_dims(100, 50))
        .with_scene_availability(SceneAvailability::Unsupported);
    assert!(matches!(
        artifact.scene_contract(),
        RenderSceneContract::Unsupported
    ));
}

#[test]
fn render_artifact_with_diagnostics_replaces_existing() {
    use puml::diagnostic::Diagnostic;
    let artifact =
        RenderArtifact::default().with_diagnostics(vec![Diagnostic::warning("test warning")]);
    assert_eq!(artifact.diagnostics.len(), 1);
}

#[test]
fn render_artifact_push_and_extend_diagnostics() {
    use puml::diagnostic::Diagnostic;
    let mut artifact = RenderArtifact::default();
    artifact.push_diagnostic(Diagnostic::warning("one"));
    artifact.extend_diagnostics(vec![
        Diagnostic::warning("two"),
        Diagnostic::warning("three"),
    ]);
    assert_eq!(artifact.diagnostics.len(), 3);
}

#[test]
fn render_artifact_media_type_is_svg_by_default() {
    let artifact = RenderArtifact::default();
    assert_eq!(artifact.media_type(), "image/svg+xml");
}

#[test]
fn render_artifact_with_common_commands_sets_scale() {
    let cmds = RenderCommonCommands::from_parts(Some(ScaleSpec::Factor(2.0)), None);
    let artifact = RenderArtifact::default().with_common_commands(cmds);
    assert!(artifact.common_commands.scale.is_some());
}

#[test]
fn render_artifact_apply_common_scale_updates_svg_dimensions() {
    let svg = svg_with_dims(100, 50);
    let mut artifact = RenderArtifact::svg_only(svg);
    artifact.common_commands.scale = Some(ScaleSpec::Factor(3.0));
    artifact.apply_common_scale_to_svg_dimensions();
    let dims = artifact.dimensions.expect("dimensions after scale");
    assert_eq!(dims.width, 300.0);
    assert_eq!(dims.height, 150.0);
}

#[test]
fn render_artifact_apply_common_scale_idempotent_when_already_applied() {
    let svg = svg_with_dims(100, 50);
    let mut artifact = RenderArtifact::svg_only(svg);
    artifact.common_commands.scale = Some(ScaleSpec::Factor(2.0));
    artifact.apply_common_scale_to_svg_dimensions();
    // Second call should be a no-op because the command is already recorded
    artifact.apply_common_scale_to_svg_dimensions();
    let dims = artifact.dimensions.expect("dimensions");
    assert_eq!(dims.width, 200.0);
    assert_eq!(dims.height, 100.0);
}

#[test]
fn render_artifact_mark_command_application_deduplicates() {
    let mut artifact = RenderArtifact::default();
    artifact.mark_common_command_application(
        CommonCommandKind::Scale,
        CommonCommandPath::ArtifactOutput,
    );
    artifact.mark_common_command_application(
        CommonCommandKind::Scale,
        CommonCommandPath::RendererEmission,
    );
    // Should only have one application for Scale
    assert_eq!(artifact.common_commands.applications.len(), 1);
}

#[test]
fn render_artifact_common_command_applied_returns_true_after_marking() {
    let mut artifact = RenderArtifact::default();
    assert!(!artifact.common_command_applied(CommonCommandKind::Scale));
    artifact.mark_common_command_application(
        CommonCommandKind::Scale,
        CommonCommandPath::ArtifactOutput,
    );
    assert!(artifact.common_command_applied(CommonCommandKind::Scale));
}

#[test]
fn render_artifact_common_command_path_returns_correct_path() {
    let mut artifact = RenderArtifact::default();
    assert!(artifact
        .common_command_path(CommonCommandKind::Scale)
        .is_none());
    artifact.mark_common_command_application(
        CommonCommandKind::Scale,
        CommonCommandPath::SvgCompatibilityBridge,
    );
    assert_eq!(
        artifact.common_command_path(CommonCommandKind::Scale),
        Some(CommonCommandPath::SvgCompatibilityBridge)
    );
}

#[test]
fn render_artifact_with_renderer_emitted_mainframe_sets_mainframe_application() {
    let artifact = RenderArtifact::default().with_renderer_emitted_mainframe(true);
    assert!(artifact.common_command_applied(CommonCommandKind::Mainframe));
    assert_eq!(
        artifact.common_command_path(CommonCommandKind::Mainframe),
        Some(CommonCommandPath::RendererEmission)
    );
}

#[test]
fn render_artifact_with_renderer_emitted_mainframe_false_leaves_empty() {
    let artifact = RenderArtifact::default().with_renderer_emitted_mainframe(false);
    assert!(!artifact.common_command_applied(CommonCommandKind::Mainframe));
}

#[test]
fn render_artifact_with_common_command_parts_applies_scale_and_mainframe() {
    let artifact = RenderArtifact::default().with_common_command_parts(
        Some(ScaleSpec::Factor(1.5)),
        Some("Title".to_string()),
        false,
    );
    assert!(artifact.common_commands.scale.is_some());
    assert!(artifact.common_commands.mainframe.is_some());
}

#[test]
fn render_artifact_with_common_commands_preserves_existing_applications() {
    let mut artifact = RenderArtifact::default();
    artifact.mark_common_command_application(
        CommonCommandKind::Mainframe,
        CommonCommandPath::RendererEmission,
    );
    let new_cmds = RenderCommonCommands::from_parts(Some(ScaleSpec::Factor(2.0)), None);
    let artifact = artifact.with_common_commands(new_cmds);
    // Previously recorded applications should be preserved
    assert!(artifact.common_command_applied(CommonCommandKind::Mainframe));
}

#[test]
fn render_artifact_require_typed_scene_for_includes_owner_in_error() {
    let artifact = RenderArtifact::default();
    let err = artifact.require_typed_scene_for("my_renderer").unwrap_err();
    assert!(
        err.message.contains("my_renderer"),
        "error message should mention the owner"
    );
}

#[test]
fn render_artifact_refresh_svg_metadata_sets_dimensions_from_svg() {
    let mut artifact = RenderArtifact {
        svg: svg_with_dims(640, 480),
        ..Default::default()
    };
    artifact.refresh_svg_metadata();
    let dims = artifact.dimensions.expect("dimensions should be set");
    assert_eq!(dims.width, 640.0);
    assert_eq!(dims.height, 480.0);
}

#[test]
fn render_artifact_refresh_svg_metadata_with_viewbox() {
    let mut artifact = RenderArtifact {
        svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\" viewBox=\"10 20 200 100\"></svg>"
            .to_string(),
        ..Default::default()
    };
    artifact.refresh_svg_metadata();
    let dims = artifact.dimensions.expect("dims");
    assert_eq!(dims.width, 200.0);
    assert_eq!(dims.height, 100.0);
    let vb = dims.view_box.expect("viewBox");
    assert_eq!(vb.origin.x, 10.0);
    assert_eq!(vb.origin.y, 20.0);
    assert_eq!(vb.size.width, 200.0);
    assert_eq!(vb.size.height, 100.0);
}

#[test]
fn render_artifact_refresh_svg_metadata_no_viewbox() {
    let mut artifact = RenderArtifact {
        svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>"
            .to_string(),
        ..Default::default()
    };
    artifact.refresh_svg_metadata();
    let dims = artifact.dimensions.expect("dims");
    assert!(dims.view_box.is_none());
}

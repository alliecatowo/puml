use puml::render::validate;
use std::fs;

use crate::harness::{render_svg, rendered_svg_path, svg_shape_failures, workspace_root, Failure};
use crate::manifest::{load_manifest, Fixture, GeometryProfile};
use crate::svg_test_helpers::SvgDoc;

/// Extract the inner-text of every `<text ...>...</text>` element in the SVG.
/// Nested `<tspan>` and similar tags are stripped so we get the raw text
/// content. Returns one entry per `<text>` element (may be empty string).
fn extract_text_contents(svg: &str) -> Vec<String> {
    let bytes = svg.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = svg[i..].find("<text") {
        let start = i + rel;
        // Require word boundary after "<text" (so we don't match "<textarea>").
        let after = start + b"<text".len();
        if after >= bytes.len() {
            break;
        }
        let next_ch = bytes[after];
        if !(next_ch == b' ' || next_ch == b'\t' || next_ch == b'>' || next_ch == b'/') {
            i = after;
            continue;
        }
        let Some(gt_rel) = svg[after..].find('>') else {
            break;
        };
        let open_end = after + gt_rel + 1;
        if bytes[open_end - 2] == b'/' {
            out.push(String::new());
            i = open_end;
            continue;
        }
        let Some(close_rel) = svg[open_end..].find("</text>") else {
            break;
        };
        let content_end = open_end + close_rel;
        let inner = &svg[open_end..content_end];
        out.push(strip_inner_tags(inner));
        i = content_end + "</text>".len();
    }
    out
}

/// Strip XML tags from the inner content of a `<text>` element, preserving
/// the visible text only.
fn strip_inner_tags(inner: &str) -> String {
    let bytes = inner.as_bytes();
    let mut out = String::with_capacity(inner.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rest = &inner[i..];
            if let Some(j) = rest.find('>') {
                i += j + 1;
                continue;
            } else {
                break;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    decode_xml_entities(out.trim())
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#10;", "\n")
}

struct FocusedTextFixture {
    path: &'static str,
    required_text: &'static [&'static str],
}

const NO_FOCUSED_TEXT_REQUIREMENTS: &[&str] = &[];

const FOCUSED_TEXT_SWEEP_FIXTURES: &[FocusedTextFixture] = &[
    FocusedTextFixture {
        path: "docs/examples/sequence/01_basic.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/sequence/05_alt_opt_loop.puml",
        required_text: &[
            "alt credentials valid",
            "invalid",
            "opt remember me",
            "loop 3 times",
            "authenticate",
            "token",
            "401 Unauthorized",
            "store session",
            "heartbeat",
            "pong",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/class/01_basic.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/class/02_inheritance.puml",
        required_text: &[
            "Vehicle",
            "Car",
            "Truck",
            "+make: String",
            "+model: String",
            "+start()",
            "+doors: Int",
            "+drive()",
            "+payload: Float",
            "+haul()",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/activity/01_simple_flow.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/activity/02_if_then_else.puml",
        required_text: &[
            "If-Then-Else Decision",
            "Receive Request",
            // Condition is inside the diamond; guard label floats on the arrow.
            "authenticated?",
            "yes",
            // "(else) no" and "(endif)" are control-flow markers (#533 fix) -
            // they drive arrow routing but are never rendered as visible text.
            "Process",
            "Return 200",
            "Return 401",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/state/01_basic.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/deployment/03_cloud.puml",
        required_text: &[
            "EC2 Instance",
            "RDS Instance",
            "S3 Bucket",
            "Lambda Function",
            "queries",
            "stores",
            "reads",
        ],
    },
    FocusedTextFixture {
        path: "docs/diagrams/architecture-overview.puml",
        required_text: &[
            "CLI",
            "LSP",
            "WASM",
            "Preprocessor",
            "Language Service",
            "Parser",
            "Renderer",
            "SVG / PNG / Text",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/sdl/02_with_transitions.puml",
        required_text: &["retry", "complete", "Idle", "Waiting", "Done"],
    },
];

const FAST_VISUAL_SMOKE_FIXTURES: &[&str] = &[
    "docs/examples/sequence/01_basic.puml",
    "docs/examples/sequence/05_alt_opt_loop.puml",
    "docs/examples/class/02_inheritance.puml",
    "docs/examples/activity/02_if_then_else.puml",
    "docs/examples/state/01_basic.puml",
    "docs/examples/component/01_basic.puml",
    "docs/examples/deployment/01_nodes.puml",
    "docs/examples/deployment/03_cloud.puml",
    "docs/examples/gantt/01_basic.puml",
    "docs/examples/mindmap/01_basic.puml",
    "docs/examples/wbs/01_basic.puml",
    "docs/examples/c4/01_context.puml",
    "docs/examples/chart/01_bar.puml",
];

fn check_fixture(fixture: &Fixture) -> Option<Failure> {
    check_fixture_with_required_text(fixture, NO_FOCUSED_TEXT_REQUIREMENTS)
}

fn check_fixture_with_required_text(
    fixture: &Fixture,
    focused_required_text: &[&str],
) -> Option<Failure> {
    let root = workspace_root();
    let path = root.join(&fixture.path);
    if !path.exists() {
        return Some(Failure {
            fixture: fixture.path.clone(),
            reasons: vec![format!(
                "fixture file not found: {} (resolve relative to workspace root)",
                path.display()
            )],
        });
    }
    let svg = match render_svg(&path) {
        Ok(s) => s,
        Err(e) => {
            return Some(Failure {
                fixture: fixture.path.clone(),
                reasons: vec![format!("render failed: {e}")],
            });
        }
    };

    // Persist the SVG to target/visual-diff/<family>/<basename>.svg for inspection.
    let artifact_path = rendered_svg_path(&root, fixture);
    if let Some(diff_dir) = artifact_path.parent() {
        let _ = fs::create_dir_all(diff_dir);
    }
    let _ = fs::write(&artifact_path, &svg);

    let mut reasons = svg_shape_failures(&svg);
    let texts = extract_text_contents(&svg);

    let empty_count = texts.iter().filter(|t| t.is_empty()).count();
    if empty_count > 0 {
        reasons.push(format!(
            "found {} empty `<text>` element(s); rendered {} non-empty out of {} total. \
             This is the missing-label bug class. Inspect the SVG at {}",
            empty_count,
            texts.len() - empty_count,
            texts.len(),
            artifact_path.display(),
        ));
    }

    let joined: String = texts.join("\n");
    for expected in &fixture.expected_text {
        if !joined.contains(expected) {
            reasons.push(format!(
                "expected text {:?} not found in any <text> element",
                expected
            ));
        }
    }
    for expected in focused_required_text {
        if !joined.contains(expected) {
            reasons.push(format!(
                "focused sweep expected text {:?} not found in any <text> element",
                expected
            ));
        }
    }
    for unexpected in &fixture.unexpected_text {
        if joined.contains(unexpected) {
            reasons.push(format!(
                "unexpected text {:?} was found in rendered <text> elements",
                unexpected
            ));
        }
    }

    let nonempty = texts.iter().filter(|t| !t.is_empty()).count();
    if nonempty < fixture.min_text_elements {
        reasons.push(format!(
            "expected >= {} non-empty <text> elements, found {}",
            fixture.min_text_elements, nonempty
        ));
    }

    append_semantic_svg_contract_failures(&svg, fixture, &mut reasons);

    if reasons.is_empty() {
        None
    } else {
        Some(Failure {
            fixture: fixture.path.clone(),
            reasons,
        })
    }
}

fn append_semantic_svg_contract_failures(svg: &str, fixture: &Fixture, reasons: &mut Vec<String>) {
    if !fixture.required_classes.is_empty()
        || !fixture.expected_counts.is_empty()
        || !fixture.required_data_attrs.is_empty()
        || fixture.geometry_profile.is_some()
    {
        let Ok(doc) = SvgDoc::try_parse(svg) else {
            reasons.push("rendered SVG did not parse as XML for semantic hook checks".to_string());
            return;
        };

        for class_name in &fixture.required_classes {
            if doc.class_count(class_name) == 0 {
                reasons.push(format!(
                    "required SVG class {:?} not found; semantic render hooks regressed",
                    class_name
                ));
            }
        }

        for (target, expected) in &fixture.expected_counts {
            let actual = count_expected_target(&doc, target);
            if !expected.accepts(actual) {
                reasons.push(format!(
                    "expected {} elements matching {target:?}, found {actual}",
                    expected.description()
                ));
            }
        }

        for attr in &fixture.required_data_attrs {
            if attr.matching_count(&doc) == 0 {
                reasons.push(format!(
                    "required SVG data attribute {} not found",
                    attr.description()
                ));
            }
        }

        append_canonical_puml_hook_failures(&doc, fixture, reasons);
        append_geometry_profile_failures(&doc, svg, fixture, reasons);
    }
}

fn append_canonical_puml_hook_failures(
    doc: &SvgDoc<'_>,
    fixture: &Fixture,
    reasons: &mut Vec<String>,
) {
    if !fixture_opts_into_canonical_puml_hooks(fixture) {
        return;
    }

    for node in doc.elements_with_class_any_tag("puml-node") {
        for attr in ["data-puml-id", "data-puml-kind", "data-puml-bbox"] {
            if node.attribute(attr).is_none() {
                reasons.push(format!(
                    "canonical .puml-node hook is missing required {attr:?}"
                ));
            }
        }
    }

    for edge in doc.elements_with_class_any_tag("puml-edge") {
        for attr in ["data-puml-from", "data-puml-to"] {
            if edge.attribute(attr).is_none() {
                reasons.push(format!(
                    "canonical .puml-edge hook is missing required {attr:?}"
                ));
            }
        }
    }

    for label in doc.elements_with_class_any_tag("puml-label") {
        for attr in ["data-puml-owner", "data-puml-label-kind", "data-puml-bbox"] {
            if label.attribute(attr).is_none() {
                reasons.push(format!(
                    "canonical .puml-label hook is missing required {attr:?}"
                ));
            }
        }
    }
}

fn fixture_opts_into_canonical_puml_hooks(fixture: &Fixture) -> bool {
    fixture
        .required_classes
        .iter()
        .any(|class| class.starts_with("puml-"))
        || fixture
            .expected_counts
            .keys()
            .any(|target| count_target_name(target).is_some_and(|name| name.starts_with("puml-")))
        || fixture
            .required_data_attrs
            .iter()
            .any(|attr| attr.name().starts_with("data-puml-"))
}

fn append_geometry_profile_failures(
    doc: &SvgDoc<'_>,
    svg: &str,
    fixture: &Fixture,
    reasons: &mut Vec<String>,
) {
    match fixture.geometry_profile {
        None | Some(GeometryProfile::StructuralOnly | GeometryProfile::Unsupported) => {}
        Some(
            GeometryProfile::Basic
            | GeometryProfile::Chart
            | GeometryProfile::Tree
            | GeometryProfile::Timeline,
        ) => {
            append_semantic_bbox_failures(svg, fixture.geometry_profile.unwrap(), reasons);
        }
        Some(GeometryProfile::Graph) => {
            append_semantic_bbox_failures(svg, GeometryProfile::Graph, reasons);
            let node_count = doc.class_count("puml-node") + doc.class_count("uml-node");
            let edge_count = doc.class_count("puml-edge") + doc.class_count("uml-relation");
            if node_count == 0 || edge_count == 0 {
                reasons.push(format!(
                    "graph geometry profile requires puml/uml node and edge hooks; found {node_count} node hook(s), {edge_count} edge hook(s)"
                ));
                return;
            }

            let edge_node = validate::check_edge_node_clearance(svg);
            let endpoints = validate::check_endpoint_connectivity(svg);
            if edge_node.is_empty() && endpoints.is_empty() {
                return;
            }

            let details = edge_node
                .iter()
                .chain(endpoints.iter())
                .take(3)
                .map(|violation| violation.message.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            reasons.push(format!(
                "graph geometry profile failed: {} edge/node violation(s), {} endpoint violation(s)",
                edge_node.len(),
                endpoints.len()
            ));
            if !details.is_empty() {
                reasons.push(format!("first geometry violations: {details}"));
            }
        }
    }
}

fn append_semantic_bbox_failures(svg: &str, profile: GeometryProfile, reasons: &mut Vec<String>) {
    let semantic = validate::check_semantic_bboxes_inside_viewbox(svg);
    if semantic.is_empty() {
        return;
    }

    let details = semantic
        .iter()
        .take(3)
        .map(|violation| violation.message.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    reasons.push(format!(
        "{} geometry profile failed: {} semantic bbox violation(s)",
        geometry_profile_name(profile),
        semantic.len()
    ));
    if !details.is_empty() {
        reasons.push(format!(
            "first {} geometry violations: {details}",
            geometry_profile_name(profile)
        ));
    }
}

fn geometry_profile_name(profile: GeometryProfile) -> &'static str {
    match profile {
        GeometryProfile::Basic => "basic",
        GeometryProfile::Graph => "graph",
        GeometryProfile::Chart => "chart",
        GeometryProfile::Tree => "tree",
        GeometryProfile::Timeline => "timeline",
        GeometryProfile::StructuralOnly => "structural-only",
        GeometryProfile::Unsupported => "unsupported",
    }
}

fn count_expected_target(doc: &SvgDoc<'_>, target: &str) -> usize {
    if let Some(class_name) = target
        .strip_prefix("class:")
        .or_else(|| target.strip_prefix('.'))
    {
        return doc.class_count(class_name);
    }
    if let Some(attr_name) = target
        .strip_prefix("attr:")
        .or_else(|| target.strip_prefix("data_attr:"))
    {
        return doc.attr_count(attr_name);
    }
    if let Some(tag) = target.strip_prefix("tag:") {
        return doc.tag_count(tag);
    }
    doc.class_count(target)
}

fn count_target_name(target: &str) -> Option<&str> {
    if target.starts_with("attr:") || target.starts_with("data_attr:") || target.starts_with("tag:")
    {
        None
    } else {
        target
            .strip_prefix("class:")
            .or_else(|| target.strip_prefix('.'))
            .or(Some(target))
    }
}

fn run_text_sweep<'a>(fixtures: impl IntoIterator<Item = &'a Fixture>, total: usize) {
    let mut failures: Vec<Failure> = Vec::new();
    for fixture in fixtures {
        if let Some(f) = check_fixture(fixture) {
            failures.push(f);
        }
    }
    if !failures.is_empty() {
        let mut report = String::new();
        report.push_str(&format!(
            "\nVisual regression: {}/{} fixtures failed\n",
            failures.len(),
            total
        ));
        for f in &failures {
            report.push_str(&format!("\n  FIXTURE: {}\n", f.fixture));
            for r in &f.reasons {
                report.push_str(&format!("    - {}\n", r));
            }
        }
        report.push_str(
            "\nRendered SVGs are written to target/visual-diff/<family>/<fixture>.svg\n\
             for inspection. See tests/visual_regression/README.md for how to add\n\
             or update fixtures.\n",
        );
        panic!("{report}");
    }
}

#[test]
fn visual_regression_focused_text_presence_sweep() {
    let manifest = load_manifest();
    let mut failures: Vec<Failure> = Vec::new();

    for focused_fixture in FOCUSED_TEXT_SWEEP_FIXTURES {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.path == focused_fixture.path)
            .unwrap_or_else(|| {
                panic!(
                    "focused visual text sweep fixture {} must exist in manifest",
                    focused_fixture.path
                )
            });
        if let Some(failure) =
            check_fixture_with_required_text(fixture, focused_fixture.required_text)
        {
            failures.push(failure);
        }
    }

    if !failures.is_empty() {
        let total = FOCUSED_TEXT_SWEEP_FIXTURES.len();
        let mut report = String::new();
        report.push_str(&format!(
            "\nFocused visual regression: {}/{} fixtures failed\n",
            failures.len(),
            total
        ));
        for f in &failures {
            report.push_str(&format!("\n  FIXTURE: {}\n", f.fixture));
            for r in &f.reasons {
                report.push_str(&format!("    - {}\n", r));
            }
        }
        report.push_str(
            "\nRendered SVGs are written to target/visual-diff/<family>/<fixture>.svg\n\
             for inspection. See tests/visual_regression/README.md for how to add\n\
             or update fixtures.\n",
        );
        panic!("{report}");
    }
}

#[test]
fn visual_smoke_representative_docs_examples_matrix() {
    let manifest = load_manifest();
    let mut fixtures = Vec::new();

    for path in FAST_VISUAL_SMOKE_FIXTURES {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.path == *path)
            .unwrap_or_else(|| panic!("fast visual smoke fixture {path} must exist in manifest"));
        fixtures.push(fixture);
    }

    run_text_sweep(fixtures, FAST_VISUAL_SMOKE_FIXTURES.len());
}

#[test]
fn visual_regression_all_fixtures() {
    let manifest = load_manifest();
    run_text_sweep(manifest.fixtures.iter(), manifest.fixtures.len());
}

#[test]
fn text_extractor_handles_nested_tspan() {
    let svg = r#"<svg><text x="0" y="0"><tspan>Hello</tspan> <tspan>World</tspan></text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Hello World");
}

#[test]
fn text_extractor_handles_self_closing() {
    let svg = r#"<svg><text/><text x="0">Visible</text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0], "");
    assert_eq!(texts[1], "Visible");
}

#[test]
fn text_extractor_ignores_textarea() {
    let svg = r#"<svg><textarea>noise</textarea><text x="0">Real</text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Real");
}

#[test]
fn text_extractor_decodes_entities() {
    let svg = r#"<svg><text>Foo &amp; Bar &lt;baz&gt;</text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts[0], "Foo & Bar <baz>");
}

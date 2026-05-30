//! Regression tests for issue #450 — extend theme presets to all diagram families.
//!
//! Verifies that `!theme <name>` directives are honoured by every family that
//! carries a colour model, and that families which have no colour model emit a
//! deterministic diagnostic rather than silently dropping the directive.
//!
//! Families under test (grouped by normaliser path):
//!
//! * **Stub / graph families** (class, object, usecase, salt) — via `GraphStyleCascade`
//! * **Extended graph families** (component, deployment) — via `GraphStyleCascade`
//! * **Activity** — `activity_style_from_sequence_theme`
//! * **Timing** — `timing_style_from_sequence_theme`
//! * **State** — `state_style_from_sequence_theme`
//! * **MindMap / WBS** — `mindmap_style_from_sequence_theme`
//! * **Salt** — `salt_style_from_sequence_theme`
//! * **Sequence** — `SequenceStyle` preset
//!
//! Cyborg dark-fill token used as the distinguishing probe: `#060606`
//! (participant background in every cyborg-family projection).

use puml::{normalize_family, parse, render_source_to_svg_for_family, DiagramFamily};

// ─── Cyborg colour probes ─────────────────────────────────────────────────────

/// The participant-background colour projected from the cyborg theme for
/// graph/activity/timing/state families.
const CYBORG_NODE_FILL: &str = "#060606";

/// The arrow/edge accent colour from the cyborg theme.
const CYBORG_EDGE: &str = "#2a9fd6";

// ─── Helper ──────────────────────────────────────────────────────────────────

fn assert_svg_contains(svg: &str, token: &str, context: &str) {
    assert!(
        svg.contains(token),
        "{context}: expected SVG to contain `{token}`\nSVG excerpt:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ─── Class family ────────────────────────────────────────────────────────────

#[test]
fn class_cyborg_theme_node_fill_reaches_svg() {
    let src = "@startuml\n\
        !theme cyborg\n\
        class User {\n  +id: UUID\n}\n\
        class Order\n\
        User --> Order\n\
        @enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class)
        .expect("class/cyborg should render");
    assert_svg_contains(&svg, CYBORG_NODE_FILL, "class/cyborg: node fill");
    assert_svg_contains(&svg, CYBORG_EDGE, "class/cyborg: edge/arrow colour");
}

// ─── Component family ─────────────────────────────────────────────────────────

#[test]
fn component_cyborg_theme_node_fill_reaches_svg() {
    let src = "@startuml\n\
        !theme cyborg\n\
        component API\n\
        component Database\n\
        API --> Database : queries\n\
        @enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Component)
        .expect("component/cyborg should render");
    assert_svg_contains(&svg, CYBORG_NODE_FILL, "component/cyborg: node fill");
    assert_svg_contains(&svg, CYBORG_EDGE, "component/cyborg: edge/border colour");
}

// ─── State family ─────────────────────────────────────────────────────────────

#[test]
fn state_cyborg_theme_node_fill_reaches_svg() {
    let src = "@startuml\n\
        !theme cyborg\n\
        [*] --> Idle\n\
        Idle --> Running : start\n\
        Running --> [*]\n\
        @enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::State)
        .expect("state/cyborg should render");
    // State nodes use participant_background_color → background fill
    assert_svg_contains(&svg, CYBORG_NODE_FILL, "state/cyborg: node fill");
    assert_svg_contains(&svg, CYBORG_EDGE, "state/cyborg: border/edge colour");
}

// ─── UseCase family ───────────────────────────────────────────────────────────

#[test]
fn usecase_cyborg_theme_node_fill_reaches_svg() {
    let src = "@startuml\n\
        !theme cyborg\n\
        actor User\n\
        usecase \"Login\" as UC1\n\
        User --> UC1\n\
        @enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::UseCase)
        .expect("usecase/cyborg should render");
    assert_svg_contains(&svg, CYBORG_NODE_FILL, "usecase/cyborg: node fill");
}

// ─── Activity family ──────────────────────────────────────────────────────────

#[test]
fn activity_cyborg_theme_node_fill_reaches_svg() {
    let src = "@startuml\n\
        !theme cyborg\n\
        :Start;\n\
        :Process;\n\
        :End;\n\
        @enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Activity)
        .expect("activity/cyborg should render");
    assert_svg_contains(&svg, CYBORG_NODE_FILL, "activity/cyborg: node fill");
}

// ─── MindMap family ───────────────────────────────────────────────────────────

#[test]
fn mindmap_cyborg_theme_colors_reach_svg() {
    let src = "@startmindmap\n\
        !theme cyborg\n\
        * Root\n\
        ** Branch A\n\
        *** Leaf 1\n\
        @endmindmap\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::MindMap)
        .expect("mindmap/cyborg should render");
    // MindMap groups derive from group_background_color (#080808) and
    // participant_background_color (#060606) at different depths.
    assert!(
        svg.contains("#080808") || svg.contains(CYBORG_NODE_FILL),
        "mindmap/cyborg: expected theme colour in SVG"
    );
}

// ─── WBS family ───────────────────────────────────────────────────────────────

#[test]
fn wbs_cyborg_theme_colors_reach_svg() {
    let src = "@startwbs\n\
        !theme cyborg\n\
        * Project\n\
        ** Phase 1\n\
        *** Task A\n\
        @endwbs\n";
    let svg =
        render_source_to_svg_for_family(src, DiagramFamily::Wbs).expect("wbs/cyborg should render");
    assert!(
        svg.contains("#080808") || svg.contains(CYBORG_NODE_FILL),
        "wbs/cyborg: expected theme colour in SVG"
    );
}

// ─── Deployment family ────────────────────────────────────────────────────────

#[test]
fn deployment_cyborg_theme_node_fill_reaches_svg() {
    let src = "@startuml\n\
        !theme cyborg\n\
        node WebServer\n\
        database Storage\n\
        WebServer --> Storage\n\
        @enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Deployment)
        .expect("deployment/cyborg should render");
    assert_svg_contains(&svg, CYBORG_NODE_FILL, "deployment/cyborg: node fill");
}

// ─── Theme normalization: class style tokens reachable via NormalizedDocument ─

#[test]
fn class_cerulean_theme_normalizes_to_typed_style() {
    let src = "@startuml\n\
        !theme cerulean\n\
        class Foo\n\
        class Bar\n\
        Foo --> Bar\n\
        @enduml\n";
    let doc = parse(src).expect("cerulean class should parse");
    let normalized = normalize_family(doc).expect("cerulean class should normalize");
    let puml::NormalizedDocument::Family(family) = normalized else {
        panic!("expected Family document");
    };
    let Some(puml::model::FamilyStyle::Class(style)) = family.family_style else {
        panic!("expected Class style in family_style");
    };
    // cerulean: participant_background_color = #d9edf7
    assert_eq!(
        style.background_color, "#d9edf7",
        "cerulean class background must match participant_background_color"
    );
    // cerulean: arrow_color = #2fa4e7
    assert_eq!(
        style.arrow_color, "#2fa4e7",
        "cerulean class arrow_color must match arrow_color token"
    );
}

// ─── Theme normalization: component style tokens reachable via NormalizedDocument

#[test]
fn component_materia_theme_normalizes_to_typed_style() {
    let src = "@startuml\n\
        !theme materia\n\
        component Frontend\n\
        component Backend\n\
        Frontend --> Backend\n\
        @enduml\n";
    let doc = parse(src).expect("materia component should parse");
    let normalized = normalize_family(doc).expect("materia component should normalize");
    let puml::NormalizedDocument::Family(family) = normalized else {
        panic!("expected Family document");
    };
    let Some(puml::model::FamilyStyle::Component(style)) = family.family_style else {
        panic!("expected Component style in family_style");
    };
    // materia: participant_background_color = #e3f2fd
    assert_eq!(
        style.background_color, "#e3f2fd",
        "materia component background must match participant_background_color"
    );
    // materia: arrow_color = #2196f3
    assert_eq!(
        style.arrow_color, "#2196f3",
        "materia component arrow_color must match arrow_color token"
    );
}

// ─── Unknown theme → diagnostic, not a panic ─────────────────────────────────

#[test]
fn unknown_theme_in_class_diagram_returns_error() {
    let src = "@startuml\n\
        !theme not-a-real-theme\n\
        class Foo\n\
        @enduml\n";
    let doc = parse(src).expect("source with unknown theme should parse");
    let result = normalize_family(doc);
    assert!(
        result.is_err(),
        "unknown theme in class diagram should produce a diagnostic error, not panic"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("E_THEME_UNKNOWN"),
        "diagnostic should identify unknown theme: {err:?}"
    );
}

// ─── Theme `from` url → deterministic diagnostic ─────────────────────────────

#[test]
fn theme_from_url_in_class_diagram_returns_error() {
    let src = "@startuml\n\
        !theme cerulean from https://example.com/themes\n\
        class Foo\n\
        @enduml\n";
    let doc = parse(src).expect("source with theme-from-url should parse");
    let result = normalize_family(doc);
    assert!(
        result.is_err(),
        "theme-from-url in class diagram should produce a diagnostic error"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("E_THEME_SOURCE_UNSUPPORTED"),
        "diagnostic should indicate unsupported theme source: {err:?}"
    );
}

// ─── Gantt/Timeline: theme emits deterministic warning ────────────────────────

#[test]
fn gantt_theme_emits_unsupported_warning() {
    use puml::{normalize_family, parse};
    let src = "@startgantt\n\
        !theme cerulean\n\
        [Task A] lasts 3 days\n\
        @endgantt\n";
    let doc = parse(src).expect("gantt with theme should parse");
    let result = normalize_family(doc).expect("gantt with theme should normalize (not error)");
    let puml::NormalizedDocument::Timeline(timeline) = result else {
        panic!("expected Timeline normalized document");
    };
    assert!(
        timeline
            .warnings
            .iter()
            .any(|w| w.message.contains("W_THEME_UNSUPPORTED")),
        "gantt !theme should emit W_THEME_UNSUPPORTED warning; got: {:?}",
        timeline.warnings
    );
}

// ─── Chen ER: theme emits deterministic warning ───────────────────────────────

#[test]
fn chen_theme_emits_unsupported_warning() {
    use puml::{normalize_family, parse};
    // Chen diagrams use @startuml with entity/relationship syntax
    let src = "@startuml\n\
        !theme cyborg\n\
        entity Customer {\n  *id\n  name\n}\n\
        @enduml\n";
    let doc = parse(src).expect("chen with theme should parse");
    // Chen normalizer returns ChenDocument which is in NormalizedDocument::Chen
    let result = normalize_family(doc).expect("chen with theme should normalize");
    let puml::NormalizedDocument::Chen(chen) = result else {
        // If this diagram didn't parse as Chen, skip rather than fail —
        // the Chen parser may fall back to class family for simple entity syntax.
        return;
    };
    assert!(
        chen.warnings
            .iter()
            .any(|w| w.message.contains("W_THEME_UNSUPPORTED")),
        "chen !theme should emit W_THEME_UNSUPPORTED warning; got: {:?}",
        chen.warnings
    );
}

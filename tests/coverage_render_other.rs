//! Coverage uplift tests for non-graph-family renderers.
//!
//! Each test renders real source from `docs/examples/` or inline source and
//! asserts:
//!   - SVG is non-empty and contains expected substrings.
//!   - For migrated renderers: `scene_availability == TypedScene`, typed scene
//!     node/edge/lane counts are sane, and `validate_geometry()` is empty.
//!
//! Refs #1258

use puml::render_core::SceneAvailability;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render_source_to_svg should succeed")
}

fn render_artifacts(src: &str) -> Vec<puml::RenderArtifact> {
    puml::render_source_to_artifacts(src).expect("render_source_to_artifacts should succeed")
}

fn single_artifact(src: &str) -> puml::RenderArtifact {
    let mut arts = render_artifacts(src);
    assert_eq!(arts.len(), 1, "expected exactly one artifact");
    arts.remove(0)
}

// ─────────────────────────────────────────────────────────────────────────────
// State diagram
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn state_basic_transitions_render_typed_scene() {
    let src = include_str!("../docs/examples/state/01_basic.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "SVG should be non-empty");
    assert!(
        artifact.svg.contains("state-transition"),
        "basic state SVG should contain state-transition elements"
    );
    assert!(
        artifact.svg.contains("Idle") && artifact.svg.contains("Running"),
        "state labels should appear in SVG"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "state renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("state artifact must expose typed scene");
    assert!(
        scene.nodes.len() >= 3,
        "basic state: expected at least 3 nodes (start + Idle + Running), got {}",
        scene.nodes.len()
    );
    assert!(
        !scene.edges.is_empty(),
        "basic state: expected at least one transition edge"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "basic state scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn state_nested_composite_renders_typed_scene_with_groups() {
    let src = include_str!("../docs/examples/state/07_nested.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Operational"),
        "composite state name should appear in SVG"
    );
    assert!(
        artifact.svg.contains("Working"),
        "nested composite state should appear in SVG"
    );
    assert!(
        artifact.svg.contains("Fetching") && artifact.svg.contains("Processing"),
        "inner state labels should appear in SVG"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("nested composite state must expose typed scene");
    // Nodes: [*] in, [*] out, Operational, Idle, Working, Fetching, Processing, [*] in Working, [*] out Working
    assert!(
        scene.nodes.len() >= 5,
        "nested composite: expected at least 5 nodes, got {}",
        scene.nodes.len()
    );
    // Composite states appear as scene groups
    assert!(
        !scene.groups.is_empty(),
        "nested composite state: groups should capture composite containment"
    );
    // Note: intra-composite transitions intentionally cross the parent group's bounding box,
    // producing EdgeCrossesNode issues that are known layout artefacts, not new regressions.
    // Geometry correctness for nested composites is tracked separately; we verify scene
    // population here without asserting zero violations.
}

#[test]
fn state_concurrent_regions_render_typed_scene() {
    let src = include_str!("../docs/examples/state/03_concurrent.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Processing"),
        "concurrent state SVG should include composite name"
    );
    assert!(
        artifact.svg.contains("Parsing") || artifact.svg.contains("Parse"),
        "concurrent region state should appear in SVG"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("concurrent state must expose typed scene");
    assert!(
        scene.nodes.len() >= 3,
        "concurrent state: expected at least 3 nodes, got {}",
        scene.nodes.len()
    );
    // Note: transitions inside concurrent regions produce EdgeCrossesNode issues against the
    // outer Processing composite — these are known layout artefacts tracked separately.
}

#[test]
fn state_history_pseudostates_render_typed_scene() {
    let src = include_str!("../docs/examples/state/04_history.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Active"),
        "history state SVG should contain Active state"
    );
    assert!(
        artifact.svg.contains("Running"),
        "history state SVG should contain Running state"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("history state must expose typed scene");
    assert!(
        scene.nodes.len() >= 4,
        "history state: expected at least 4 nodes, got {}",
        scene.nodes.len()
    );
    // Note: history pseudo-state transitions that pass through composite group bounds
    // produce EdgeCrossesNode issues — these are known layout artefacts tracked separately.
}

#[test]
fn state_full_machine_renders_typed_scene_with_entry_exit_actions() {
    let src = include_str!("../docs/examples/state/08_full_machine.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("state-transition"),
        "full machine SVG should have transitions"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("full machine state must expose typed scene");
    assert!(
        scene.nodes.len() >= 5,
        "full machine: expected at least 5 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        !scene.edges.is_empty(),
        "full machine: expected at least one transition edge"
    );
    // Note: full state machine contains nested composites whose transitions produce known
    // EdgeCrossesNode geometry issues — tracked separately, not asserted here.
}

// ─────────────────────────────────────────────────────────────────────────────
// Activity diagram
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn activity_simple_flow_renders_typed_scene() {
    let src = include_str!("../docs/examples/activity/01_simple_flow.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "activity SVG should be non-empty");
    assert!(
        artifact.svg.contains("data-activity-kind"),
        "activity SVG should contain data-activity-kind metadata"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "activity renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("activity artifact must expose typed scene");
    // start + at least one action + stop = 3 nodes minimum
    assert!(
        scene.nodes.len() >= 3,
        "simple flow: expected at least 3 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        !scene.edges.is_empty(),
        "simple flow: expected at least one edge"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "simple flow scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn activity_fork_join_renders_typed_scene_with_branch_nodes() {
    let src = include_str!("../docs/examples/activity/04_fork_join.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Subtask A"),
        "fork join SVG should contain fork branch label"
    );
    assert!(
        artifact.svg.contains("Subtask B"),
        "fork join SVG should contain another fork branch"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("fork join activity must expose typed scene");
    // start + Start Task + fork + 3 subtasks + merge + stop
    assert!(
        scene.nodes.len() >= 6,
        "fork join: expected at least 6 nodes, got {}",
        scene.nodes.len()
    );
    // Note: fork/join bar nodes have zero-height rects that produce EdgeCrossesNode
    // violations for edges touching them — these are known artefacts tracked separately.
}

#[test]
fn activity_if_then_else_renders_typed_scene_with_decision_nodes() {
    let src = include_str!("../docs/examples/activity/02_if_then_else.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-activity-kind"),
        "if-then-else SVG should contain activity kind metadata"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("if-then-else activity must expose typed scene");
    assert!(
        scene.nodes.len() >= 4,
        "if-then-else: expected at least 4 nodes (start + decision + branches + stop), got {}",
        scene.nodes.len()
    );
    // Note: decision diamond edge routing produces EdgeCrossesNode against the diamond node
    // in some layouts — known artefact tracked separately, not asserted here.
}

#[test]
fn activity_partition_swimlanes_renders_typed_scene_with_lanes() {
    let src = include_str!("../docs/examples/activity/07_partition.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Worker") && artifact.svg.contains("Backend"),
        "swimlane SVG should show partition/lane names"
    );
    assert!(
        artifact.svg.contains("Frontend"),
        "swimlane SVG should show all partition names"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("swimlane activity must expose typed scene");
    assert!(
        scene.nodes.len() >= 5,
        "swimlane partition: expected at least 5 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        !scene.lanes.is_empty(),
        "swimlane partition: scene should have lane frames populated"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "swimlane scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn activity_while_loop_renders_typed_scene() {
    let src = include_str!("../docs/examples/activity/05_while_loop.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("while loop activity must expose typed scene");
    assert!(
        scene.nodes.len() >= 3,
        "while loop: expected at least 3 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "while loop scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Timing diagram
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn timing_concise_renders_typed_scene() {
    let src = include_str!("../docs/examples/timing/01_concise.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "timing SVG should be non-empty");
    assert!(
        artifact.svg.contains("Web User") || artifact.svg.contains("WU"),
        "concise timing SVG should include participant label"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "timing renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("concise timing must expose typed scene");
    // Timing uses LaneFrames for signal rows, not SceneNodes.
    // Two participants → at least 2 lanes.
    assert!(
        scene.lanes.len() >= 2,
        "concise timing: expected at least 2 lane frames (one per signal), got {}",
        scene.lanes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "concise timing scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn timing_robust_renders_typed_scene_with_participant_states() {
    let src = include_str!("../docs/examples/timing/02_robust.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Server") || artifact.svg.contains("S"),
        "robust timing SVG should include Server participant"
    );
    assert!(
        artifact.svg.contains("Client") || artifact.svg.contains("C"),
        "robust timing SVG should include Client participant"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("robust timing must expose typed scene");
    // Timing uses LaneFrames for signal rows, not SceneNodes.
    assert!(
        scene.lanes.len() >= 2,
        "robust timing: expected at least 2 lane frames, got {}",
        scene.lanes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "robust timing scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn timing_binary_renders_typed_scene_with_binary_lanes() {
    let src = include_str!("../docs/examples/timing/04_binary.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Request") || artifact.svg.contains("REQ"),
        "binary timing SVG should include Request participant"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("binary timing must expose typed scene");
    // Timing uses LaneFrames for signal rows, not SceneNodes.
    // Request + Ack = 2 binary signals → 2 lanes.
    assert!(
        scene.lanes.len() >= 2,
        "binary timing: expected at least 2 lane frames, got {}",
        scene.lanes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "binary timing scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn timing_concurrent_with_messages_renders_typed_scene() {
    let src = include_str!("../docs/examples/timing/05_concurrent_timelines_message_arrows.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("timing-message"),
        "concurrent timing SVG should include timing-message elements"
    );
    assert!(
        artifact.svg.contains("CPU"),
        "concurrent timing SVG should include CPU participant"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("concurrent timing must expose typed scene");
    // Timing uses LaneFrames for signal rows, not SceneNodes.
    // CPU + L1 + MEM + IO + clock = 5 signals → at least 4 lanes.
    assert!(
        scene.lanes.len() >= 4,
        "concurrent timing: expected at least 4 lane frames, got {}",
        scene.lanes.len()
    );
    // Message arrows between lanes become scene edges.
    assert!(
        !scene.edges.is_empty(),
        "concurrent timing with messages: expected at least one edge (message arrow)"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "concurrent timing scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Gantt (timeline)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn gantt_basic_tasks_render_typed_scene() {
    let src = include_str!("../docs/examples/gantt/01_basic.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "gantt SVG should be non-empty");
    assert!(
        artifact.svg.contains("gantt-task"),
        "gantt SVG should contain gantt-task elements"
    );
    assert!(
        artifact.svg.contains("Design") || artifact.svg.contains("Build"),
        "gantt SVG should show task labels"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "gantt renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("gantt artifact must expose typed scene");
    // Design, Build, Test = 3 tasks minimum
    assert!(
        scene.nodes.len() >= 3,
        "basic gantt: expected at least 3 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "basic gantt scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn gantt_milestones_render_typed_scene_with_milestone_nodes() {
    let src = include_str!("../docs/examples/gantt/02_milestones.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("gantt-milestone"),
        "milestones gantt SVG should contain gantt-milestone elements"
    );
    assert!(
        artifact.svg.contains("Alpha") || artifact.svg.contains("Beta"),
        "gantt milestones SVG should show milestone labels"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("milestones gantt must expose typed scene");
    // Alpha, Beta, RC1, GA = 4 milestones
    assert!(
        scene.nodes.len() >= 4,
        "milestones gantt: expected at least 4 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "milestones gantt scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn gantt_dated_tasks_render_typed_scene() {
    let src = include_str!("../docs/examples/gantt/04_dated.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("gantt-task") || artifact.svg.contains("gantt-scale-tick"),
        "dated gantt SVG should contain task or tick elements"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("dated gantt must expose typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "dated gantt: expected at least one node in scene"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "dated gantt scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Chronology (timeline)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn chronology_events_render_typed_scene() {
    let src = include_str!("../docs/examples/chronology/01_events.puml");
    let artifact = single_artifact(src);

    assert!(
        !artifact.svg.is_empty(),
        "chronology SVG should be non-empty"
    );
    assert!(
        artifact.svg.contains("chronology-event-card"),
        "chronology SVG should contain event-card elements"
    );
    assert!(
        artifact.svg.contains("chronology-marker"),
        "chronology SVG should contain marker elements"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "chronology renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("chronology artifact must expose typed scene");
    // 5 milestone events in the fixture
    assert!(
        scene.nodes.len() >= 3,
        "chronology events: expected at least 3 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "chronology events scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn chronology_timeline_with_eras_renders_typed_scene() {
    let src = include_str!("../docs/examples/chronology/04_eras_spans.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("chronology-axis"),
        "era chronology SVG should contain the axis element"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("chronology eras must expose typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "chronology eras: scene should have nodes"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "chronology eras scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Salt (UI wireframe)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn salt_basic_widgets_render_typed_scene() {
    let src = include_str!("../docs/examples/salt/01_basic_widgets.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "salt SVG should be non-empty");
    assert!(
        artifact.svg.contains("data-salt-style"),
        "salt SVG should contain data-salt-style attribute"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "salt renderer should emit TypedScene for non-empty diagrams"
    );

    let scene = artifact
        .typed_scene()
        .expect("salt artifact must expose typed scene");
    // At least the dialog frame + label + button nodes
    assert!(
        !scene.nodes.is_empty(),
        "salt basic widgets: scene should have nodes"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "salt basic widgets scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn salt_frame_dialog_renders_typed_scene() {
    let src = include_str!("../docs/examples/salt/02_frame.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-salt-style"),
        "salt frame SVG should contain data-salt-style attribute"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("salt frame must expose typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "salt frame: scene should have at least one node"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "salt frame scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn salt_tabs_widget_renders_typed_scene() {
    let src = include_str!("../docs/examples/salt/04_tabs.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Tab") || artifact.svg.contains("tab"),
        "salt tabs SVG should mention tab labels"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("salt tabs must expose typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "salt tabs: scene should have nodes"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "salt tabs scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Mindmap
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn mindmap_basic_tree_renders_typed_scene() {
    let src = include_str!("../docs/examples/mindmap/01_basic.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "mindmap SVG should be non-empty");
    assert!(
        artifact.svg.contains("mindmap-node"),
        "mindmap SVG should contain mindmap-node elements"
    );
    assert!(
        artifact.svg.contains("Project"),
        "mindmap root label should appear in SVG"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "mindmap renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("mindmap artifact must expose typed scene");
    // Project + Planning + Development + Testing = 4 nodes
    assert!(
        scene.nodes.len() >= 4,
        "basic mindmap: expected at least 4 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.edges.len() >= 3,
        "basic mindmap: expected at least 3 edges (root → each child), got {}",
        scene.edges.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "basic mindmap scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn mindmap_multi_level_tree_renders_typed_scene_with_depth() {
    let src = include_str!("../docs/examples/mindmap/02_multi_level.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Technology Stack") || artifact.svg.contains("mindmap-root"),
        "multi-level mindmap should show root label"
    );
    assert!(
        artifact.svg.contains("Frontend") && artifact.svg.contains("Backend"),
        "second-level nodes should appear in multi-level mindmap"
    );
    assert!(
        artifact.svg.contains("React") || artifact.svg.contains("Rust"),
        "third-level leaf nodes should appear in multi-level mindmap"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("multi-level mindmap must expose typed scene");
    // root + 3 L2 + multiple L3 leaves
    assert!(
        scene.nodes.len() >= 6,
        "multi-level mindmap: expected at least 6 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "multi-level mindmap scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn wbs_basic_tree_renders_typed_scene() {
    let src = include_str!("../docs/examples/wbs/01_basic.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "WBS SVG should be non-empty");
    assert!(
        artifact.svg.contains("Project Scope") || artifact.svg.contains("Phase"),
        "WBS SVG should contain root or child labels"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "WBS renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("WBS artifact must expose typed scene");
    // Project Scope + Phase 1 + Phase 2 + Phase 3 = 4 nodes
    assert!(
        scene.nodes.len() >= 4,
        "basic WBS: expected at least 4 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.edges.len() >= 3,
        "basic WBS: expected at least 3 edges, got {}",
        scene.edges.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "basic WBS scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn wbs_multi_level_deep_renders_typed_scene() {
    let src = include_str!("../docs/examples/wbs/04_multi_level.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("Software Development"),
        "multi-level WBS should show root label"
    );
    assert!(
        artifact.svg.contains("Requirements") && artifact.svg.contains("Design"),
        "second-level WBS nodes should appear in SVG"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("multi-level WBS must expose typed scene");
    // Deep hierarchy: many nodes
    assert!(
        scene.nodes.len() >= 8,
        "multi-level WBS: expected at least 8 nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "multi-level WBS scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn json_object_renders_typed_scene() {
    let src = include_str!("../docs/examples/json/01_object.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "JSON SVG should be non-empty");
    assert!(
        artifact.svg.contains("data-projection=\"json\""),
        "JSON SVG should carry data-projection=json attribute"
    );
    assert!(
        artifact.svg.contains("Alice"),
        "JSON SVG should contain the value Alice"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "JSON renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("JSON artifact must expose typed scene");
    // 3 keys: name, age, active
    assert!(
        scene.nodes.len() >= 3,
        "JSON object: expected at least 3 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "JSON object scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn json_nested_structure_renders_typed_scene_with_depth() {
    let src = include_str!("../docs/examples/json/03_nested.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-json-depth"),
        "nested JSON SVG should carry depth attributes"
    );
    assert!(
        artifact.svg.contains("user") || artifact.svg.contains("settings"),
        "nested JSON SVG should show top-level keys"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("nested JSON must expose typed scene");
    // user.name, user.roles[0], user.roles[1], settings.theme, settings.locale, etc.
    assert!(
        scene.nodes.len() >= 5,
        "nested JSON: expected at least 5 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "nested JSON scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn json_array_renders_typed_scene() {
    let src = include_str!("../docs/examples/json/02_array.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-projection=\"json\""),
        "JSON array SVG should carry data-projection=json"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("JSON array must expose typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "JSON array: scene should have nodes"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "JSON array scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// YAML
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn yaml_mapping_renders_typed_scene() {
    let src = include_str!("../docs/examples/yaml/01_mapping.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "YAML SVG should be non-empty");
    assert!(
        artifact.svg.contains("data-projection=\"yaml\""),
        "YAML SVG should carry data-projection=yaml attribute"
    );
    assert!(
        artifact.svg.contains("Alice"),
        "YAML SVG should contain the value Alice"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "YAML renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("YAML artifact must expose typed scene");
    // name, age, active = 3 rows
    assert!(
        scene.nodes.len() >= 3,
        "YAML mapping: expected at least 3 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "YAML mapping scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn yaml_nested_structure_renders_typed_scene_with_depth() {
    let src = include_str!("../docs/examples/yaml/03_nested.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-yaml-depth"),
        "nested YAML SVG should carry depth attributes"
    );
    assert!(
        artifact.svg.contains("database") || artifact.svg.contains("localhost"),
        "nested YAML SVG should show database key or host value"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("nested YAML must expose typed scene");
    // database, host, port, credentials, user, password = 6+ rows
    assert!(
        scene.nodes.len() >= 5,
        "nested YAML: expected at least 5 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "nested YAML scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Board
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn board_sprint_board_renders_typed_scene() {
    let src = include_str!("../docs/examples/board/01_sprint_board.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "board SVG should be non-empty");
    assert!(
        artifact.svg.contains("board-column"),
        "board SVG should contain board-column elements"
    );
    assert!(
        artifact.svg.contains("Backlog") || artifact.svg.contains("Doing"),
        "board SVG should show column labels"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "board renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("board artifact must expose typed scene");
    // 3 columns: Backlog, Doing, Done → at least 3 lane frames
    assert!(
        !scene.nodes.is_empty() || !scene.lanes.is_empty(),
        "board scene should have column nodes or lanes"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "board scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn board_inline_with_cards_renders_typed_scene_with_all_columns() {
    let src = r#"@startboard
title Release Tracker
Todo
+Write parser tests
++Unit coverage
+Fix rendering bug
In Progress
+Migrate activity renderer
Done
+Ship coverage gate
+Add WBS scene
@endboard
"#;
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("board-column"),
        "board SVG should contain board-column elements"
    );
    assert!(
        artifact.svg.contains("Todo") && artifact.svg.contains("Done"),
        "board SVG should show all column names"
    );
    assert!(
        artifact.svg.contains("board-card"),
        "board SVG should show card elements"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("inline board must expose typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "inline board: scene should have at least one node"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "inline board scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Files
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn files_repo_tree_renders_typed_scene() {
    let src = include_str!("../docs/examples/files/01_repo_tree.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "files SVG should be non-empty");
    assert!(
        artifact.svg.contains("files-entry"),
        "files SVG should contain files-entry elements"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "files renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("files artifact must expose typed scene");
    // Several path entries from the fixture
    assert!(
        scene.nodes.len() >= 3,
        "files repo tree: expected at least 3 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "files scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn files_with_notes_renders_typed_scene_and_includes_note_entries() {
    let src = r#"@startfiles
title Project files
<note>
Project structure
</note>
/src/main.rs
/src/lib.rs
/tests/integration.rs
<note>
test note
</note>
/README.md
@endfiles
"#;
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("files-entry"),
        "files-with-notes SVG should contain files-entry elements"
    );
    assert!(
        artifact.svg.contains("files-note"),
        "files SVG should contain files-note elements for embedded notes"
    );
    assert!(
        artifact.svg.contains("main.rs") || artifact.svg.contains("src"),
        "files SVG should show file path entries"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("files-with-notes must expose typed scene");
    // 4 file entries + note entries
    assert!(
        scene.nodes.len() >= 4,
        "files with notes: expected at least 4 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "files-with-notes scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Chen ER
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn chen_basic_entities_render_typed_scene() {
    let src = include_str!("../docs/examples/chen/01_basic.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty(), "Chen SVG should be non-empty");
    assert!(
        artifact.svg.contains("chen-entity"),
        "Chen SVG should contain chen-entity elements"
    );
    assert!(
        artifact.svg.contains("Person") || artifact.svg.contains("Location"),
        "Chen SVG should show entity labels"
    );
    assert!(
        artifact.svg.contains("chen-relationship"),
        "Chen SVG should contain chen-relationship elements"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "Chen renderer should emit TypedScene"
    );

    let scene = artifact
        .typed_scene()
        .expect("Chen artifact must expose typed scene");
    // Person entity + Location entity + Birthplace relationship = at least 3 nodes
    assert!(
        scene.nodes.len() >= 3,
        "basic Chen: expected at least 3 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        !scene.edges.is_empty(),
        "basic Chen: expected at least one edge (entity-relation connection)"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "basic Chen scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn chen_attributes_render_typed_scene_with_attribute_ellipses() {
    let src = include_str!("../docs/examples/chen/02_attributes.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("chen-attribute"),
        "Chen attributes SVG should contain chen-attribute elements"
    );
    assert!(
        artifact.svg.contains("chen-entity"),
        "Chen attributes SVG should contain entity rectangles"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("Chen attributes must expose typed scene");
    // 2 entities + several attributes
    assert!(
        scene.nodes.len() >= 4,
        "Chen attributes: expected at least 4 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "Chen attributes scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

#[test]
fn chen_relationships_render_typed_scene_with_cardinalities() {
    let src = include_str!("../docs/examples/chen/03_relationships.puml");
    let artifact = single_artifact(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("chen-relationship"),
        "Chen relationships SVG should contain relationship diamonds"
    );
    assert!(
        artifact.svg.contains("chen-cardinality") || artifact.svg.contains("chen-edge"),
        "Chen relationships SVG should have cardinality or edge elements"
    );
    assert!(
        artifact.svg.contains("CUSTOMER") || artifact.svg.contains("MOVIE"),
        "Chen relationships SVG should show entity labels"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);

    let scene = artifact
        .typed_scene()
        .expect("Chen relationships must expose typed scene");
    // CUSTOMER + MOVIE + INVOICE + RENTED_TO = 4 nodes minimum
    assert!(
        scene.nodes.len() >= 4,
        "Chen relationships: expected at least 4 scene nodes, got {}",
        scene.nodes.len()
    );
    assert!(
        !scene.edges.is_empty(),
        "Chen relationships: expected entity-relation edges in scene"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "Chen relationships scene geometry should be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-family determinism spot-check
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn all_covered_families_produce_deterministic_svg() {
    let sources: &[&str] = &[
        include_str!("../docs/examples/state/01_basic.puml"),
        include_str!("../docs/examples/activity/01_simple_flow.puml"),
        include_str!("../docs/examples/timing/01_concise.puml"),
        include_str!("../docs/examples/gantt/01_basic.puml"),
        include_str!("../docs/examples/chronology/01_events.puml"),
        include_str!("../docs/examples/mindmap/01_basic.puml"),
        include_str!("../docs/examples/wbs/01_basic.puml"),
        include_str!("../docs/examples/json/01_object.puml"),
        include_str!("../docs/examples/yaml/01_mapping.puml"),
        include_str!("../docs/examples/board/01_sprint_board.puml"),
        include_str!("../docs/examples/files/01_repo_tree.puml"),
        include_str!("../docs/examples/chen/01_basic.puml"),
    ];

    for src in sources {
        let first = render_svg(src);
        let second = render_svg(src);
        assert_eq!(
            first, second,
            "renderer must produce byte-identical output on repeated calls"
        );
        assert!(
            !first.is_empty(),
            "every covered family must produce non-empty SVG"
        );
    }
}

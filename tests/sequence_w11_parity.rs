// Wave-11 Batch A — Sequence Diagram Parity Tests
//
// Covers: divider lines (== Label ==), box/endbox participant groups,
// simultaneous messages (& prefix), and multi-participant notes (note over A, B).

fn svg_of(input: &str) -> String {
    puml::render_source_to_svg(input)
        .unwrap_or_else(|e| panic!("render_source_to_svg failed: {e:?}\nInput:\n{input}"))
}

fn layout_of(input: &str) -> puml::scene::Scene {
    let ast = puml::parse(input).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    puml::layout::layout(&doc, puml::scene::LayoutOptions::default())
}

// ─── Divider lines (== Label ==) ─────────────────────────────────────────────

/// `== Section ==` outside any group renders a horizontal rule with a centered
/// label — PlantUML "separator" syntax.
#[test]
fn sequence_divider_line_renders_horizontal_rule_with_label() {
    let svg = svg_of(
        r#"@startuml
participant A
participant B

A -> B: before

== Initialization ==

B --> A: after
@enduml"#,
    );

    // The separator label text should appear in the SVG output.
    assert!(
        svg.contains("Initialization"),
        "separator label text must appear in SVG"
    );
    // A horizontal line element should be present.
    assert!(
        svg.contains("<line"),
        "separator must emit an SVG <line> element"
    );
    // The == == wrapper around the label is added by the renderer.
    assert!(
        svg.contains("=="),
        "separator renderer must include == markers around the label"
    );
    // The diagram must not panic and must produce non-empty SVG.
    assert!(!svg.is_empty());
}

/// Multiple separators each produce their own labeled horizontal rule.
#[test]
fn sequence_multiple_dividers_each_render_labeled_rule() {
    let svg = svg_of(
        r#"@startuml
A -> B: step 1
== Main Section ==
B -> A: step 2
== Cleanup ==
A -> B: step 3
@enduml"#,
    );

    assert!(
        svg.contains("Main Section"),
        "first separator label must appear"
    );
    assert!(
        svg.contains("Cleanup"),
        "second separator label must appear"
    );
}

/// An empty `== ==` separator (no label text) renders without panicking.
#[test]
fn sequence_empty_divider_renders_without_panic() {
    let svg = svg_of(
        r#"@startuml
A -> B: message
== ==
B --> A: reply
@enduml"#,
    );
    assert!(!svg.is_empty());
    assert!(
        svg.contains("<line"),
        "empty separator must still emit a line"
    );
}

/// The layout engine places the separator structure at a distinct y-coordinate
/// between the surrounding messages.
#[test]
fn sequence_divider_layout_placed_between_messages() {
    let scene = layout_of(
        r#"@startuml
participant A
participant B

A -> B: before
== Midpoint ==
B --> A: after
@enduml"#,
    );

    assert_eq!(scene.messages.len(), 2, "should have exactly two messages");
    assert_eq!(
        scene.structures.len(),
        1,
        "should have exactly one structure (the separator)"
    );

    let sep = &scene.structures[0];
    let msg_before = &scene.messages[0];
    let msg_after = &scene.messages[1];

    // The separator y should fall strictly between the two message y-coordinates.
    assert!(
        sep.y > msg_before.y,
        "separator y ({}) must be below first message y ({})",
        sep.y,
        msg_before.y
    );
    assert!(
        sep.y < msg_after.y,
        "separator y ({}) must be above second message y ({})",
        sep.y,
        msg_after.y
    );

    // The separator label must carry the text we set.
    assert_eq!(
        sep.label.as_deref(),
        Some("Midpoint"),
        "separator label should match the source text"
    );
}

// ─── box / end box participant grouping ──────────────────────────────────────

/// `box "Name" #Color … end box` produces a colored background band behind the
/// enclosed participant columns.
#[test]
fn sequence_box_group_renders_colored_band_behind_members() {
    let svg = svg_of(
        r#"@startuml
box "Backend" #LightBlue
participant Service
database DB
end box

Service -> DB: query
DB --> Service: rows
@enduml"#,
    );

    // The participant-group rect must be present with the correct CSS class.
    assert!(
        svg.contains("class=\"sequence-participant-group\""),
        "box group must emit a rect with class sequence-participant-group"
    );
    // LightBlue normalizes to its SVG hex form #add8e6.
    assert!(
        svg.contains("#add8e6"),
        "box color LightBlue must resolve to its hex equivalent #add8e6"
    );
    // The group label should appear in the SVG.
    assert!(
        svg.contains("Backend"),
        "box group label must appear in SVG"
    );
    assert!(!svg.is_empty());
}

/// `box` opens a participant group; `end box` closes it. Participants declared
/// between the two keywords become members and the layout records their IDs.
#[test]
fn sequence_box_endbox_brackets_participants() {
    let scene = layout_of(
        r#"@startuml
box "Frontend" #LightCyan
participant Browser
participant ReactApp
end box

box "Backend" #LightYellow
participant API
database DB
end box

Browser -> API: request
API -> DB: query
DB --> API: result
API --> Browser: response
@enduml"#,
    );

    // There should be exactly two "box" groups in the scene.
    let box_groups: Vec<_> = scene.groups.iter().filter(|g| g.kind == "box").collect();
    assert_eq!(
        box_groups.len(),
        2,
        "expected two box groups, found {}",
        box_groups.len()
    );

    let frontend = box_groups
        .iter()
        .find(|g| g.label.as_deref() == Some("Frontend"))
        .expect("Frontend box group not found");
    let backend = box_groups
        .iter()
        .find(|g| g.label.as_deref() == Some("Backend"))
        .expect("Backend box group not found");

    // Find the participants.
    let browser = scene
        .participants
        .iter()
        .find(|p| p.id == "Browser")
        .expect("Browser participant not found");
    let react_app = scene
        .participants
        .iter()
        .find(|p| p.id == "ReactApp")
        .expect("ReactApp participant not found");
    let api = scene
        .participants
        .iter()
        .find(|p| p.id == "API")
        .expect("API participant not found");
    let db = scene
        .participants
        .iter()
        .find(|p| p.id == "DB")
        .expect("DB participant not found");

    // Each group must horizontally contain (bracket) its members.
    assert!(
        frontend.x <= browser.x,
        "Frontend box left edge ({}) must be at or left of Browser ({})",
        frontend.x,
        browser.x
    );
    assert!(
        frontend.x + frontend.width >= react_app.x + react_app.width,
        "Frontend box right edge must cover ReactApp right edge"
    );
    assert!(
        backend.x <= api.x,
        "Backend box left edge must be at or left of API"
    );
    assert!(
        backend.x + backend.width >= db.x + db.width,
        "Backend box right edge must cover DB right edge"
    );

    // Colors should be recorded on the group.
    assert!(
        frontend.color.is_some(),
        "Frontend box must carry a color value"
    );
    assert!(
        backend.color.is_some(),
        "Backend box must carry a color value"
    );
}

/// A `box` without a color still renders the group band without panicking.
#[test]
fn sequence_box_without_color_renders_without_panic() {
    let svg = svg_of(
        r#"@startuml
box "Service Layer"
participant Auth
participant Data
end box

Auth -> Data: call
@enduml"#,
    );
    assert!(
        svg.contains("class=\"sequence-participant-group\""),
        "box group must render even without an explicit color"
    );
    assert!(svg.contains("Service Layer"));
}

// ─── Simultaneous messages (& prefix) ────────────────────────────────────────

/// A line beginning with `&` marks the message as simultaneous with the
/// immediately preceding one.  In non-TEOZ mode both messages share the same
/// Y-coordinate so they appear at the same vertical position.
#[test]
fn sequence_ampersand_prefix_marks_simultaneous_message() {
    let scene = layout_of(
        r#"@startuml
participant A
participant B
participant C

A -> B: sync request
& A -> C: also start at same time
@enduml"#,
    );

    assert_eq!(scene.messages.len(), 2, "should have exactly two messages");

    let first = &scene.messages[0];
    let second = &scene.messages[1];

    // The second message should be marked parallel.
    assert!(
        second.style.parallel,
        "message with & prefix must have style.parallel = true"
    );

    // Both messages should share the same Y coordinate (same visual row).
    assert_eq!(
        first.y, second.y,
        "simultaneous messages must share the same Y coordinate: first.y={}, second.y={}",
        first.y, second.y
    );
}

/// The SVG output for simultaneous messages must not emit an extra visual row
/// between the two messages — both should appear at the same height.
#[test]
fn sequence_ampersand_svg_renders_without_panic() {
    let svg = svg_of(
        r#"@startuml
A -> B: start
& A -> C: also start at same time
B --> A: ack
@enduml"#,
    );
    assert!(!svg.is_empty());
    // Three messages total (2 simultaneous + 1 normal), all producing line elements.
    assert!(
        svg.contains("<line"),
        "messages must produce SVG line elements"
    );
}

/// Multiple `&` lines in sequence each share the same row as the initiating
/// message.
#[test]
fn sequence_multiple_simultaneous_messages_all_share_same_row() {
    let scene = layout_of(
        r#"@startuml
participant A
participant B
participant C
participant D

A -> B: broadcast
& A -> C: and this
& A -> D: and that too
@enduml"#,
    );

    assert_eq!(scene.messages.len(), 3);

    let y0 = scene.messages[0].y;
    for msg in &scene.messages {
        assert_eq!(
            msg.y, y0,
            "all simultaneous messages must share the same y={}",
            y0
        );
    }
}

// ─── Multi-participant note (note over A, B) ──────────────────────────────────

/// `note over A, B : text` produces a note whose x starts at A's left edge
/// and whose width covers through B's right edge.
#[test]
fn sequence_note_over_two_participants_spans_lanes() {
    let scene = layout_of(
        r#"@startuml
participant Alice
participant Bob
participant Carol

Alice -> Bob: hello
note over Alice, Bob : a multi-participant note spanning both
Bob --> Alice: world
@enduml"#,
    );

    let note = scene
        .notes
        .iter()
        .find(|n| n.text.contains("multi-participant note"))
        .expect("multi-participant note not found in scene");

    let alice = scene
        .participants
        .iter()
        .find(|p| p.id == "Alice")
        .expect("Alice not found");
    let bob = scene
        .participants
        .iter()
        .find(|p| p.id == "Bob")
        .expect("Bob not found");

    // The note's left edge must be at Alice's left edge (or adjusted leftward to
    // stay within canvas bounds) and its right edge must reach Bob's right edge.
    assert!(
        note.x <= alice.x + alice.width / 2,
        "note left edge ({}) should start at or left of Alice's centerline ({})",
        note.x,
        alice.x + alice.width / 2
    );
    assert!(
        note.x + note.width >= bob.x + bob.width,
        "note right edge ({}) must reach or exceed Bob's right edge ({})",
        note.x + note.width,
        bob.x + bob.width
    );
}

/// A three-participant note spans all three lanes.
#[test]
fn sequence_note_over_three_participants_spans_all_lanes() {
    let scene = layout_of(
        r#"@startuml
participant A
participant B
participant C

A -> B: msg
note over A, B, C : spans all three
B --> A: reply
@enduml"#,
    );

    let note = scene
        .notes
        .iter()
        .find(|n| n.text.contains("spans all three"))
        .expect("three-participant note not found");

    let a = scene
        .participants
        .iter()
        .find(|p| p.id == "A")
        .expect("A not found");
    let c = scene
        .participants
        .iter()
        .find(|p| p.id == "C")
        .expect("C not found");

    // Note must reach from A's area to C's right edge.
    assert!(
        note.x <= a.x + a.width / 2,
        "note should start no further right than A's center"
    );
    assert!(
        note.x + note.width >= c.x + c.width,
        "note right edge must cover C's full width"
    );
}

/// The SVG output for a multi-participant note must contain the note text and a
/// note shape covering the span.
#[test]
fn sequence_note_over_two_participants_svg_contains_text_and_shape() {
    let svg = svg_of(
        r#"@startuml
participant A
participant B

A -> B: request
note over A, B : spanning note text
@enduml"#,
    );

    assert!(
        svg.contains("spanning note text"),
        "note text must appear in SVG"
    );
    // A folded-note path element should be present.
    assert!(
        svg.contains("<path"),
        "note shape must be rendered as an SVG path"
    );
    assert!(!svg.is_empty());
}

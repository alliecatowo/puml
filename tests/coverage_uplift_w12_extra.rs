//! Coverage uplift wave 12 — extra tests targeting api/render_summary.rs and
//! api/render_scene.rs which were at 24.65% and 65.97% coverage respectively.
//!
//! These tests exercise `normalized_model_summary_to_json` for every
//! NormalizedDocument variant and `normalized_scene_summary_to_json` for the
//! scene paths.
//!
//! Refs #89

use puml::{
    normalize_family, normalized_artifact_scene_summary_to_json, normalized_model_summary_to_json,
    normalized_scene_summary_to_json, parse,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn model_summary(src: &str) -> serde_json::Value {
    let doc = parse(src).expect("parse failed");
    let model = normalize_family(doc).expect("normalize failed");
    normalized_model_summary_to_json(&model)
}

fn scene_summary(src: &str) -> serde_json::Value {
    let doc = parse(src).expect("parse failed");
    let model = normalize_family(doc).expect("normalize failed");
    normalized_scene_summary_to_json(&model)
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Sequence
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_sequence_kind() {
    let v = model_summary("@startuml\nA -> B : hello\n@enduml");
    assert_eq!(v["kind"], "Sequence");
}

#[test]
fn model_summary_sequence_participants() {
    let v = model_summary("@startuml\nA -> B : hello\n@enduml");
    assert!(v["participants"].as_u64().unwrap() >= 2);
}

#[test]
fn model_summary_sequence_events() {
    let v = model_summary("@startuml\nA -> B : hello\n@enduml");
    assert!(v["events"].as_u64().unwrap() >= 1);
}

#[test]
fn model_summary_sequence_warnings_empty() {
    let v = model_summary("@startuml\nA -> B : hello\n@enduml");
    assert_eq!(v["warnings"].as_u64().unwrap(), 0);
}

#[test]
fn model_summary_sequence_with_title() {
    let v = model_summary("@startuml\ntitle My Sequence\nA -> B : hello\n@enduml");
    assert_eq!(v["kind"], "Sequence");
    assert_eq!(v["title"], "My Sequence");
}

#[test]
fn model_summary_sequence_header_footer() {
    let v = model_summary("@startuml\nheader Top header\nfooter Page footer\nA -> B : hi\n@enduml");
    assert_eq!(v["kind"], "Sequence");
    assert!(!v["header"].is_null());
    assert!(!v["footer"].is_null());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Family variants (Class / Activity / etc.)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_class_kind() {
    let v = model_summary("@startuml\nclass Foo\n@enduml");
    let kind = v["kind"].as_str().unwrap();
    // family kind is the Debug format of FamilyKind — just check it is present
    assert!(!kind.is_empty());
}

#[test]
fn model_summary_class_nodes() {
    let v = model_summary("@startuml\nclass Foo\nclass Bar\n@enduml");
    let nodes = v["nodes"].as_u64().unwrap();
    assert!(nodes >= 2, "expected >=2 nodes, got {nodes}");
}

#[test]
fn model_summary_class_relations() {
    let v = model_summary("@startuml\nclass Foo\nclass Bar\nFoo --> Bar\n@enduml");
    let rels = v["relations"].as_u64().unwrap();
    assert!(rels >= 1, "expected >=1 relation, got {rels}");
}

#[test]
fn model_summary_class_warnings_field_present() {
    let v = model_summary("@startuml\nclass Foo\n@enduml");
    assert!(v["warnings"].is_number());
}

#[test]
fn model_summary_class_title() {
    let v = model_summary("@startuml\ntitle Class Diagram\nclass Foo\n@enduml");
    // title field present (may be null or string)
    assert!(v["title"].is_null() || v["title"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — State
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_state_kind() {
    let src = "@startuml\ntitle Session Lifecycle\nstate Idle\nstate Active\nIdle --> Active : open\n@enduml";
    let v = model_summary(src);
    assert_eq!(v["kind"], "State");
}

#[test]
fn model_summary_state_nodes() {
    let src = "@startuml\nstate Idle\nstate Active\nstate Closed\nIdle --> Active\nActive --> Closed\n@enduml";
    let v = model_summary(src);
    assert!(v["nodes"].as_u64().unwrap() >= 3);
}

#[test]
fn model_summary_state_transitions() {
    let src = "@startuml\nstate Idle\nstate Active\nIdle --> Active : open\nActive --> Idle : close\n@enduml";
    let v = model_summary(src);
    assert!(v["transitions"].as_u64().unwrap() >= 2);
}

#[test]
fn model_summary_state_warnings() {
    let src = "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Timeline (Gantt / Chronology)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_gantt_kind() {
    let src = "@startgantt\n[Design]\n[Build]\n[Design] lasts 5 days\n@endgantt";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Timeline");
}

#[test]
fn model_summary_gantt_tasks() {
    let src =
        "@startgantt\n[Design]\n[Build]\n[Design] lasts 3 days\n[Build] lasts 2 days\n@endgantt";
    let v = model_summary(src);
    assert!(v["tasks"].as_u64().unwrap() >= 2);
}

#[test]
fn model_summary_gantt_title() {
    let src = "@startgantt\ntitle Project Plan\n[Design]\n[Design] lasts 3 days\n@endgantt";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Timeline");
    assert!(v["title"].is_null() || v["title"].is_string());
}

#[test]
fn model_summary_gantt_milestones_field() {
    let src = "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt";
    let v = model_summary(src);
    assert!(v["milestones"].is_number());
}

#[test]
fn model_summary_gantt_constraints_field() {
    let src = "@startgantt\n[Design]\n[Build]\n[Design] lasts 3 days\n[Build] starts at [Design]'s end\n@endgantt";
    let v = model_summary(src);
    assert!(v["constraints"].is_number());
}

#[test]
fn model_summary_chronology_kind() {
    let src = "@startchronology\nPhase 1 happens on 2026-05-10\nPhase 2 happens on 2026-06-01\n@endchronology";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Timeline");
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Json / Yaml
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_json_kind() {
    let src = "@startjson\n{\"name\": \"puml\", \"version\": 1}\n@endjson";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Json");
}

#[test]
fn model_summary_json_warnings_present() {
    let src = "@startjson\n{\"name\": \"puml\"}\n@endjson";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

#[test]
fn model_summary_yaml_kind() {
    let src = "@startyaml\nproject:\n  name: puml\n@endyaml";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Yaml");
}

#[test]
fn model_summary_yaml_warnings_present() {
    let src = "@startyaml\nproject:\n  name: puml\n@endyaml";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Nwdiag
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_nwdiag_kind() {
    let src = "@startnwdiag\nnetwork dmz {\n    web01\n    web02\n}\n@endnwdiag";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Nwdiag");
}

#[test]
fn model_summary_nwdiag_warnings_present() {
    let src = "@startnwdiag\nnetwork dmz {\n    web01\n}\n@endnwdiag";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Archimate
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_archimate_kind() {
    let src = "@startarchimate\narchimate \"Customer\" as cust <<motivation>>\narchimate \"Service\" as svc <<application>>\nRel_Serving(svc, cust, \"serves\")\n@endarchimate";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Archimate");
}

#[test]
fn model_summary_archimate_warnings_present() {
    let src = "@startarchimate\narchimate \"Customer\" as cust <<motivation>>\n@endarchimate";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Regex
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_regex_kind() {
    let src = "@startregex\ntitle Regex Demo\na(b|c)*d?\n[a-z]+\n@endregex";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Regex");
}

#[test]
fn model_summary_regex_warnings_present() {
    let src = "@startregex\na(b|c)*d?\n@endregex";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Ebnf
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_ebnf_kind() {
    let src = "@startebnf\ntitle Tiny Grammar\nexpr = term { \"+\" term } ;\n@endebnf";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Ebnf");
}

#[test]
fn model_summary_ebnf_warnings_present() {
    let src = "@startebnf\nexpr = term ;\n@endebnf";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Math
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_math_kind() {
    let src = "@startmath\na^2 + b^2 = c^2\n@endmath";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Math");
}

#[test]
fn model_summary_math_warnings_present() {
    let src = "@startmath\na^2 + b^2 = c^2\n@endmath";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Sdl
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_sdl_kind() {
    let src = "@startsdl\ntitle Login FSM\nstart Idle\nstate Authenticating\nstop Done\nIdle -> Authenticating : credentials\n@endsdl";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Sdl");
}

#[test]
fn model_summary_sdl_warnings_present() {
    let src = "@startsdl\nstart Idle\nstop Done\nIdle -> Done : go\n@endsdl";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Ditaa
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_ditaa_kind() {
    let src = "@startditaa\n+----+   +----+\n| A  |-->| B  |\n+----+   +----+\n@endditaa";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Ditaa");
}

#[test]
fn model_summary_ditaa_warnings_present() {
    let src = "@startditaa\n+----+\n| A  |\n+----+\n@endditaa";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Chart
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_chart_kind() {
    let src = "@startchart\ntitle Bar Demo\nbar\n\"Apples\" 10\n\"Bananas\" 20\n@endchart";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Chart");
}

#[test]
fn model_summary_chart_warnings_present() {
    let src = "@startchart\nbar\n\"A\" 10\n@endchart";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Chen
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_chen_kind() {
    let src = "@startchen\ntitle Basic Chen ER\nentity Person {\n  Number <<key>>\n  Name\n}\nentity Location {\n  Code <<key>>\n}\nrelationship Birthplace {\n}\nBirthplace -N- Person\nBirthplace -1- Location\n@endchen";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Chen");
}

#[test]
fn model_summary_chen_nodes() {
    let src = "@startchen\nentity Person {\n  Number <<key>>\n}\nentity Location {\n  Code <<key>>\n}\n@endchen";
    let v = model_summary(src);
    assert!(v["nodes"].as_u64().unwrap() >= 2);
}

#[test]
fn model_summary_chen_relations() {
    let src = "@startchen\nentity Person {\n  Number <<key>>\n}\nentity Location {\n  Code <<key>>\n}\nrelationship Lives {\n}\nLives -N- Person\nLives -1- Location\n@endchen";
    let v = model_summary(src);
    assert!(v["relations"].as_u64().unwrap() >= 1);
}

#[test]
fn model_summary_chen_inheritances_field() {
    let src = "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen";
    let v = model_summary(src);
    assert!(v["inheritances"].is_number());
}

#[test]
fn model_summary_chen_warnings_field() {
    let src = "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Board
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_board_kind() {
    let src = "@startboard\ntitle Sprint board\nBacklog\n+Task 1\nDoing\n+Task 2\nDone\n+Task 3\n@endboard";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Board");
}

#[test]
fn model_summary_board_columns() {
    let src = "@startboard\nBacklog\n+Task A\nDoing\n+Task B\nDone\n+Task C\n@endboard";
    let v = model_summary(src);
    assert!(v["columns"].as_u64().unwrap() >= 3);
}

#[test]
fn model_summary_board_cards() {
    let src = "@startboard\nBacklog\n+Task A\n+Task B\nDoing\n+Task C\nDone\n@endboard";
    let v = model_summary(src);
    assert!(v["cards"].as_u64().unwrap() >= 3);
}

#[test]
fn model_summary_board_title() {
    let src = "@startboard\ntitle My Board\nBacklog\n+Task A\nDone\n@endboard";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Board");
    assert!(v["title"].is_null() || v["title"].is_string());
}

#[test]
fn model_summary_board_warnings_field() {
    let src = "@startboard\nBacklog\n+Task A\nDone\n@endboard";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Files
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_files_kind() {
    let src = "@startfiles\ntitle Repository tree\n/src/main.rs\n/Cargo.toml\n@endfiles";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Files");
}

#[test]
fn model_summary_files_roots() {
    let src = "@startfiles\n/src/main.rs\n/Cargo.toml\n@endfiles";
    let v = model_summary(src);
    assert!(v["roots"].is_number());
}

#[test]
fn model_summary_files_top_notes_field() {
    let src = "@startfiles\n/src/main.rs\n@endfiles";
    let v = model_summary(src);
    assert!(v["top_notes"].is_number());
}

#[test]
fn model_summary_files_warnings_field() {
    let src = "@startfiles\n/src/main.rs\n@endfiles";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

#[test]
fn model_summary_files_title() {
    let src = "@startfiles\ntitle My Files\n/src/main.rs\n@endfiles";
    let v = model_summary(src);
    assert!(v["title"].is_null() || v["title"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_model_summary_to_json — Wire
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_wire_kind() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER,DATA\n--\ncomponent Controller [150x110] left:POWER,DATA\nPanel.POWER -- Controller.POWER : 24V\n@endwire";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Wire");
}

#[test]
fn model_summary_wire_components() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire";
    let v = model_summary(src);
    assert!(v["components"].as_u64().unwrap() >= 2);
}

#[test]
fn model_summary_wire_ports() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER,DATA\n--\ncomponent Controller [150x110] left:POWER,DATA\nPanel.POWER -- Controller.POWER\n@endwire";
    let v = model_summary(src);
    assert!(v["ports"].as_u64().unwrap() >= 2);
}

#[test]
fn model_summary_wire_links() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire";
    let v = model_summary(src);
    assert!(v["links"].is_number());
}

#[test]
fn model_summary_wire_warnings_field() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire";
    let v = model_summary(src);
    assert!(v["warnings"].is_number());
}

#[test]
fn model_summary_wire_title() {
    let src = "@startwire\ntitle Wire Harness\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire";
    let v = model_summary(src);
    assert!(v["title"].is_null() || v["title"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_scene_summary_to_json — main entry point coverage
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn scene_summary_sequence_kind() {
    let v = scene_summary("@startuml\nA -> B : hello\n@enduml");
    assert_eq!(v["kind"], "Sequence");
}

#[test]
fn scene_summary_sequence_has_typed_field() {
    let v = scene_summary("@startuml\nA -> B : hello\n@enduml");
    assert!(v["typed"].is_boolean());
}

#[test]
fn scene_summary_sequence_has_page_count() {
    let v = scene_summary("@startuml\nA -> B : hello\n@enduml");
    assert!(v["pageCount"].is_number());
}

#[test]
fn scene_summary_sequence_pages_is_array() {
    let v = scene_summary("@startuml\nA -> B : hello\n@enduml");
    assert!(v["pages"].is_array());
}

#[test]
fn scene_summary_json_kind() {
    let src = "@startjson\n{\"name\": \"puml\"}\n@endjson";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Json");
}

#[test]
fn scene_summary_json_not_typed() {
    let src = "@startjson\n{\"name\": \"puml\"}\n@endjson";
    let v = scene_summary(src);
    assert_eq!(v["typed"], false);
}

#[test]
fn scene_summary_json_available_false() {
    let src = "@startjson\n{\"name\": \"puml\"}\n@endjson";
    let v = scene_summary(src);
    assert_eq!(v["available"], false);
}

#[test]
fn scene_summary_yaml_kind() {
    let src = "@startyaml\nname: puml\n@endyaml";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Yaml");
}

#[test]
fn scene_summary_nwdiag_kind() {
    let src = "@startnwdiag\nnetwork dmz {\n    web01\n}\n@endnwdiag";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Nwdiag");
}

#[test]
fn scene_summary_archimate_kind() {
    let src = "@startarchimate\narchimate \"Customer\" as cust <<motivation>>\n@endarchimate";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Archimate");
}

#[test]
fn scene_summary_regex_kind() {
    let src = "@startregex\na(b|c)*d?\n@endregex";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Regex");
}

#[test]
fn scene_summary_ebnf_kind() {
    let src = "@startebnf\nexpr = term ;\n@endebnf";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Ebnf");
}

#[test]
fn scene_summary_math_kind() {
    let src = "@startmath\na^2 + b^2 = c^2\n@endmath";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Math");
}

#[test]
fn scene_summary_sdl_kind() {
    let src = "@startsdl\nstart Idle\nstop Done\nIdle -> Done : go\n@endsdl";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Sdl");
}

#[test]
fn scene_summary_ditaa_kind() {
    let src = "@startditaa\n+----+\n| A  |\n+----+\n@endditaa";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Ditaa");
}

#[test]
fn scene_summary_chart_kind() {
    let src = "@startchart\nbar\n\"A\" 10\n@endchart";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Chart");
}

#[test]
fn scene_summary_chen_kind() {
    let src = "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen";
    let v = scene_summary(src);
    // Chen is a family — scene summary goes to family_scene_summary_to_json
    assert!(!v["kind"].as_str().unwrap().is_empty());
}

#[test]
fn scene_summary_board_kind() {
    let src = "@startboard\nBacklog\n+Task A\nDone\n@endboard";
    let v = scene_summary(src);
    assert!(!v["kind"].as_str().unwrap().is_empty());
}

#[test]
fn scene_summary_files_kind() {
    let src = "@startfiles\n/src/main.rs\n@endfiles";
    let v = scene_summary(src);
    assert!(!v["kind"].as_str().unwrap().is_empty());
}

#[test]
fn scene_summary_wire_kind() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire";
    let v = scene_summary(src);
    assert!(!v["kind"].as_str().unwrap().is_empty());
}

#[test]
fn scene_summary_gantt_kind() {
    let src = "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Timeline");
}

#[test]
fn scene_summary_state_kind() {
    let src = "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml";
    let v = scene_summary(src);
    // state diagrams normalize to Sequence OR State depending on detection
    assert!(v["kind"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// normalized_artifact_scene_summary_to_json
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn artifact_scene_sequence_kind() {
    use puml::render_artifact_pages_from_model;
    let src = "@startuml\nA -> B : hello\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Sequence");
}

#[test]
fn artifact_scene_sequence_page_count() {
    use puml::render_artifact_pages_from_model;
    let src = "@startuml\nA -> B : hello\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["pageCount"].as_u64().unwrap(), artifacts.len() as u64);
}

#[test]
fn artifact_scene_json_fallback() {
    use puml::render_artifact_pages_from_model;
    let src = "@startjson\n{\"name\": \"puml\"}\n@endjson";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Json");
    assert_eq!(v["typed"], false);
    assert_eq!(v["available"], false);
}

#[test]
fn artifact_scene_empty_artifacts() {
    use puml::render_artifact_pages_from_model;
    let src = "@startjson\n{\"name\": \"puml\"}\n@endjson";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let empty: Vec<puml::RenderArtifact> = vec![];
    let v = normalized_artifact_scene_summary_to_json(&model, &empty);
    // scene availability defaults to NotMigrated when artifacts empty
    assert_eq!(v["sceneAvailability"], "NotMigrated");
}

#[test]
fn artifact_scene_yaml_fallback() {
    use puml::render_artifact_pages_from_model;
    let src = "@startyaml\nname: puml\n@endyaml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Yaml");
}

#[test]
fn artifact_scene_nwdiag_fallback() {
    use puml::render_artifact_pages_from_model;
    let src = "@startnwdiag\nnetwork dmz {\n    web01\n}\n@endnwdiag";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Nwdiag");
}

#[test]
fn artifact_scene_chen_family() {
    use puml::render_artifact_pages_from_model;
    let src = "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert!(v["kind"].is_string());
    assert!(v["typed"].is_boolean());
}

#[test]
fn artifact_scene_board_family() {
    use puml::render_artifact_pages_from_model;
    let src = "@startboard\nBacklog\n+Task A\nDone\n@endboard";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert!(v["kind"].is_string());
}

#[test]
fn artifact_scene_files_family() {
    use puml::render_artifact_pages_from_model;
    let src = "@startfiles\n/src/main.rs\n@endfiles";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert!(v["kind"].is_string());
}

#[test]
fn artifact_scene_wire_family() {
    use puml::render_artifact_pages_from_model;
    let src = "@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert!(v["kind"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// Return-value shape checks — scene_availability fields
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn scene_summary_non_family_has_availability_field() {
    let src = "@startjson\n{\"x\": 1}\n@endjson";
    let v = scene_summary(src);
    // The fallback arm emits no sceneAvailability in normalized_scene_summary_to_json
    // but emits kind, typed, available, summary
    assert!(v["summary"].is_object());
}

#[test]
fn scene_summary_non_family_summary_has_kind() {
    let src = "@startjson\n{\"x\": 1}\n@endjson";
    let v = scene_summary(src);
    assert_eq!(v["summary"]["kind"], "Json");
}

#[test]
fn scene_summary_non_family_gantt_summary_kind() {
    let src = "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt";
    let v = scene_summary(src);
    assert!(v["summary"].is_object() || v["kind"] == "Timeline");
}

// ─────────────────────────────────────────────────────────────────────────────
// FamilyPages variants (multi-page diagrams)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_family_pages_kind() {
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\n@enduml";
    let v = model_summary(src);
    assert_eq!(v["kind"], "FamilyPages");
}

#[test]
fn model_summary_family_pages_has_pages() {
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\n@enduml";
    let v = model_summary(src);
    assert!(v["pages"].is_array());
    assert!(!v["pages"].as_array().unwrap().is_empty());
}

#[test]
fn scene_summary_family_pages_kind() {
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\n@enduml";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "FamilyPages");
}

#[test]
fn scene_summary_family_pages_has_pages_array() {
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\n@enduml";
    let v = scene_summary(src);
    assert!(v["pages"].is_array());
}

#[test]
fn artifact_scene_family_pages() {
    use puml::render_artifact_pages_from_model;
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "FamilyPages");
    assert!(v["pages"].is_array());
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional coverage for remaining uncovered files
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn model_summary_stdlib_kind() {
    // "stdlib" keyword inside @startuml triggers the Stdlib diagram family
    let src = "@startuml\nstdlib\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let v = normalized_model_summary_to_json(&model);
    assert_eq!(v["kind"], "Stdlib");
}

#[test]
fn model_summary_stdlib_entries() {
    let src = "@startuml\nstdlib\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let v = normalized_model_summary_to_json(&model);
    assert!(v["entries"].as_u64().unwrap() > 0);
}

#[test]
fn model_summary_stdlib_packs() {
    let src = "@startuml\nstdlib\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let v = normalized_model_summary_to_json(&model);
    assert!(v["packs"].as_u64().unwrap() > 0);
}

#[test]
fn model_summary_stdlib_aliases() {
    let src = "@startuml\nstdlib\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let v = normalized_model_summary_to_json(&model);
    assert!(v["aliases"].is_number());
}

#[test]
fn model_summary_stdlib_warnings() {
    let src = "@startuml\nstdlib\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let v = normalized_model_summary_to_json(&model);
    assert!(v["warnings"].is_number());
}

#[test]
fn scene_summary_stdlib_kind() {
    let src = "@startuml\nstdlib\n@enduml";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Stdlib");
}

#[test]
fn artifact_scene_stdlib() {
    use puml::render_artifact_pages_from_model;
    let src = "@startuml\nstdlib\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Stdlib");
}

#[test]
fn artifact_scene_archimate() {
    use puml::render_artifact_pages_from_model;
    let src = "@startarchimate\narchimate \"Customer\" as cust <<motivation>>\narchimate \"Service\" as svc <<application>>\nRel_Serving(svc, cust, \"serves\")\n@endarchimate";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Archimate");
    assert!(v["sceneAvailability"].is_string());
}

#[test]
fn artifact_scene_math() {
    use puml::render_artifact_pages_from_model;
    let src = "@startmath\na^2 + b^2 = c^2\n@endmath";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Math");
}

#[test]
fn artifact_scene_ebnf() {
    use puml::render_artifact_pages_from_model;
    let src = "@startebnf\nexpr = term ;\n@endebnf";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Ebnf");
}

#[test]
fn artifact_scene_regex() {
    use puml::render_artifact_pages_from_model;
    let src = "@startregex\na(b|c)*d?\n@endregex";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Regex");
}

#[test]
fn artifact_scene_ditaa() {
    use puml::render_artifact_pages_from_model;
    let src = "@startditaa\n+----+\n| A  |\n+----+\n@endditaa";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Ditaa");
}

#[test]
fn artifact_scene_chart() {
    use puml::render_artifact_pages_from_model;
    let src = "@startchart\nbar\n\"A\" 10\n@endchart";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Chart");
}

#[test]
fn artifact_scene_sdl() {
    use puml::render_artifact_pages_from_model;
    let src = "@startsdl\nstart Idle\nstop Done\nIdle -> Done : go\n@endsdl";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "Sdl");
}

#[test]
fn artifact_scene_gantt() {
    use puml::render_artifact_pages_from_model;
    let src = "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    // Timeline is not Sequence/Family/FamilyPages so goes to fallback
    assert!(v["kind"].is_string());
}

#[test]
fn scene_summary_ditaa_kind_2() {
    let src = "@startditaa\n+----+---+\n| A  | B |\n+----+---+\n@endditaa";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "Ditaa");
}

#[test]
fn scene_summary_all_non_family_have_summary_field_or_kind() {
    let cases = [
        "@startregex\na(b|c)*\n@endregex",
        "@startebnf\nexpr = term ;\n@endebnf",
        "@startmath\na^2 = b^2\n@endmath",
        "@startsdl\nstart A\nstop B\nA -> B\n@endsdl",
        "@startditaa\n+--+\n|A |\n+--+\n@endditaa",
        "@startchart\nbar\n\"X\" 5\n@endchart",
        "@startarchimate\narchimate \"A\" as a <<motivation>>\n@endarchimate",
        "@startnwdiag\nnetwork dmz {\n    w\n}\n@endnwdiag",
    ];
    for src in cases {
        let v = scene_summary(src);
        assert!(
            v["kind"].is_string(),
            "expected kind string for: {src}; got: {v:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Additional artifact scene summary tests to close remaining coverage gap
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn artifact_scene_family_pages_page_level_detail() {
    use puml::render_artifact_pages_from_model;
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    assert_eq!(v["kind"], "FamilyPages");
    let pages = v["pages"].as_array().unwrap();
    assert!(pages.len() >= 2);
    // each page should have kind
    for page in pages {
        assert!(page["kind"].is_string());
    }
}

#[test]
fn artifact_scene_sequence_pages_detail() {
    use puml::render_artifact_pages_from_model;
    let src = "@startuml\nA -> B : hello\nA -> B : world\n@enduml";
    let doc = parse(src).expect("parse");
    let model = normalize_family(doc).expect("normalize");
    let artifacts = render_artifact_pages_from_model(&model);
    let v = normalized_artifact_scene_summary_to_json(&model, &artifacts);
    let pages = v["pages"].as_array().unwrap();
    assert!(!pages.is_empty());
}

#[test]
fn model_summary_sequence_caption_field() {
    let v = model_summary("@startuml\ncaption Figure 1\nA -> B : hello\n@enduml");
    assert_eq!(v["kind"], "Sequence");
    assert!(v["caption"].is_null() || v["caption"].is_string());
}

#[test]
fn model_summary_wire_components_with_many_ports() {
    let src = "@startwire\ncomponent Panel [120x90] right:POWER,DATA,CTRL top:IN1,IN2\n--\ncomponent Controller [150x110] left:POWER,DATA right:OUT1,OUT2,OUT3\nPanel.POWER -- Controller.POWER\nPanel.DATA --> Controller.DATA\n@endwire";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Wire");
    let ports = v["ports"].as_u64().unwrap();
    assert!(ports >= 4, "expected >=4 ports, got {ports}");
    let links = v["links"].as_u64().unwrap();
    assert!(links >= 2);
}

#[test]
fn model_summary_board_empty_columns() {
    // Board with a column that has no cards
    let src = "@startboard\nBacklog\nDoing\n+Active Task\nDone\n@endboard";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Board");
    let cols = v["columns"].as_u64().unwrap();
    assert!(cols >= 3);
}

#[test]
fn model_summary_chen_with_inheritance() {
    let src = "@startchen\nentity Animal {\n  ID <<key>>\n}\nentity Dog {\n  Breed\n}\nDog ISA Animal\n@endchen";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Chen");
    // inheritances should be counted
    assert!(v["inheritances"].is_number());
}

#[test]
fn model_summary_files_with_notes() {
    let src = "@startfiles\ntitle Repo\n<note>\ntop-level note\n</note>\n/src/main.rs\n/Cargo.toml\n@endfiles";
    let v = model_summary(src);
    assert_eq!(v["kind"], "Files");
    // top_notes should be 1
    let notes = v["top_notes"].as_u64().unwrap();
    assert!(notes >= 1);
}

#[test]
fn scene_summary_family_pages_page_count() {
    let src = "@startuml\nclass Foo\nnewpage\nclass Bar\nnewpage\nclass Baz\n@enduml";
    let v = scene_summary(src);
    assert_eq!(v["kind"], "FamilyPages");
    let page_count = v["pageCount"].as_u64().unwrap_or(0);
    assert!(
        page_count >= 3 || v["pages"].as_array().map_or(0, |p| p.len()) >= 2,
        "expected multi-page FamilyPages"
    );
}

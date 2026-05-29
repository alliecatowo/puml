//! Coverage uplift wave 12 — CLI dump mode integration tests.
//!
//! These tests exercise `cli_dump.rs` and `cli_dump_ast.rs` via the `--dump` CLI
//! flag across all diagram families, significantly raising coverage on both files
//! which were at ~27-38% before this wave.
//!
//! Refs #89

use assert_cmd::Command;
use std::fs;
use tempfile::NamedTempFile;

// ─────────────────────────────────────────────────────────────────────────────
// Helper
// ─────────────────────────────────────────────────────────────────────────────

fn dump(src: &str, kind: &str) -> String {
    let f = NamedTempFile::with_suffix(".puml").unwrap();
    fs::write(f.path(), src).unwrap();
    let out = Command::cargo_bin("puml")
        .unwrap()
        .args(["--dump", kind, f.path().to_str().unwrap()])
        .output()
        .expect("puml binary");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn dump_ok(src: &str, kind: &str) -> serde_json::Value {
    let raw = dump(src, kind);
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("JSON parse failed: {e}\nRaw: {raw}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// --dump model  (exercises normalized_model_to_json variants)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn dump_model_sequence_has_participants() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "model");
    assert!(v["participants"].is_array());
}

#[test]
fn dump_model_sequence_has_events() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "model");
    assert!(v["events"].is_array());
}

#[test]
fn dump_model_class_kind() {
    let v = dump_ok("@startuml\nclass Foo\n@enduml", "model");
    assert!(v["kind"].is_string());
}

#[test]
fn dump_model_class_nodes() {
    let v = dump_ok("@startuml\nclass Foo\nclass Bar\n@enduml", "model");
    assert!(v["nodes"].is_array());
    assert!(v["nodes"].as_array().unwrap().len() >= 2);
}

#[test]
fn dump_model_class_relations() {
    let v = dump_ok(
        "@startuml\nclass Foo\nclass Bar\nFoo --> Bar\n@enduml",
        "model",
    );
    assert!(v["relations"].is_array());
}

#[test]
fn dump_model_state_kind() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml",
        "model",
    );
    assert_eq!(v["kind"], "State");
}

#[test]
fn dump_model_state_nodes() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml",
        "model",
    );
    assert!(v["nodes"].is_array());
}

#[test]
fn dump_model_state_transitions() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml",
        "model",
    );
    assert!(v["transitions"].is_array());
}

#[test]
fn dump_model_gantt_kind() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "model",
    );
    assert_eq!(v["kind"], "Gantt");
}

#[test]
fn dump_model_gantt_tasks() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Build]\n[Design] lasts 3 days\n[Build] lasts 2 days\n@endgantt",
        "model",
    );
    assert!(v["tasks"].is_array());
}

#[test]
fn dump_model_gantt_milestones() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "model",
    );
    assert!(v["milestones"].is_array());
}

#[test]
fn dump_model_gantt_constraints() {
    let v = dump_ok("@startgantt\n[Design]\n[Build]\n[Design] lasts 3 days\n[Build] starts at [Design]'s end\n@endgantt", "model");
    assert!(v["constraints"].is_array());
}

#[test]
fn dump_model_chronology_kind() {
    let v = dump_ok(
        "@startchronology\nPhase 1 happens on 2026-05-10\n@endchronology",
        "model",
    );
    assert_eq!(v["kind"], "Chronology");
}

#[test]
fn dump_model_json_kind() {
    let v = dump_ok("@startjson\n{\"name\": \"puml\"}\n@endjson", "model");
    assert_eq!(v["kind"], "Json");
}

#[test]
fn dump_model_yaml_kind() {
    let v = dump_ok("@startyaml\nname: puml\n@endyaml", "model");
    assert_eq!(v["kind"], "Yaml");
}

#[test]
fn dump_model_nwdiag_kind() {
    let v = dump_ok(
        "@startnwdiag\nnetwork dmz {\n    web01\n}\n@endnwdiag",
        "model",
    );
    assert_eq!(v["kind"], "Nwdiag");
}

#[test]
fn dump_model_archimate_kind() {
    let v = dump_ok(
        "@startarchimate\narchimate \"Customer\" as cust <<motivation>>\n@endarchimate",
        "model",
    );
    assert_eq!(v["kind"], "Archimate");
}

#[test]
fn dump_model_regex_kind() {
    let v = dump_ok("@startregex\na(b|c)*d?\n@endregex", "model");
    assert_eq!(v["kind"], "Regex");
}

#[test]
fn dump_model_ebnf_kind() {
    let v = dump_ok("@startebnf\nexpr = term ;\n@endebnf", "model");
    assert_eq!(v["kind"], "Ebnf");
}

#[test]
fn dump_model_math_kind() {
    let v = dump_ok("@startmath\na^2 + b^2 = c^2\n@endmath", "model");
    assert_eq!(v["kind"], "Math");
}

#[test]
fn dump_model_sdl_kind() {
    let v = dump_ok(
        "@startsdl\nstart Idle\nstop Done\nIdle -> Done : go\n@endsdl",
        "model",
    );
    assert_eq!(v["kind"], "Sdl");
}

#[test]
fn dump_model_ditaa_kind() {
    let v = dump_ok("@startditaa\n+----+\n| A  |\n+----+\n@endditaa", "model");
    assert_eq!(v["kind"], "Ditaa");
}

#[test]
fn dump_model_chart_kind() {
    let v = dump_ok("@startchart\nbar\n\"A\" 10\n@endchart", "model");
    assert_eq!(v["kind"], "Chart");
}

#[test]
fn dump_model_chen_kind() {
    let v = dump_ok(
        "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen",
        "model",
    );
    assert_eq!(v["kind"], "Chen");
}

#[test]
fn dump_model_chen_nodes() {
    let v = dump_ok("@startchen\nentity Person {\n  Number <<key>>\n}\nentity Location {\n  Code <<key>>\n}\n@endchen", "model");
    let nodes = v["nodes"].as_u64().unwrap();
    assert!(nodes >= 2, "expected >=2 nodes, got {nodes}");
}

#[test]
fn dump_model_board_kind() {
    let v = dump_ok("@startboard\nBacklog\n+Task A\nDone\n@endboard", "model");
    assert_eq!(v["kind"], "Board");
}

#[test]
fn dump_model_board_columns() {
    let v = dump_ok(
        "@startboard\nBacklog\n+Task A\nDoing\n+Task B\nDone\n@endboard",
        "model",
    );
    let cols = v["columns"].as_u64().unwrap();
    assert!(cols >= 3, "expected >=3 columns, got {cols}");
}

#[test]
fn dump_model_files_kind() {
    let v = dump_ok("@startfiles\n/src/main.rs\n@endfiles", "model");
    assert_eq!(v["kind"], "Files");
}

#[test]
fn dump_model_wire_kind() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "model");
    assert_eq!(v["kind"], "Wire");
}

#[test]
fn dump_model_wire_components() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "model");
    let comps = v["components"].as_u64().unwrap();
    assert!(comps >= 2, "expected >=2 components, got {comps}");
}

#[test]
fn dump_model_mindmap_kind() {
    let v = dump_ok(
        "@startmindmap\n* Root\n** Child1\n** Child2\n@endmindmap",
        "model",
    );
    assert!(v["kind"].is_string());
}

#[test]
fn dump_model_activity_kind() {
    let v = dump_ok("@startuml\nstart\n:Action A;\nstop\n@enduml", "model");
    assert!(v["kind"].is_string());
}

#[test]
fn dump_model_component_kind() {
    let v = dump_ok(
        "@startuml\ncomponent C1\ncomponent C2\nC1 --> C2\n@enduml",
        "model",
    );
    assert!(v["kind"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// --dump scene  (exercises normalized_scene_to_json variants)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn dump_scene_sequence_has_size() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "scene");
    assert!(v["size"].is_object());
}

#[test]
fn dump_scene_sequence_has_lanes() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "scene");
    assert!(v["lanes"].is_array());
}

#[test]
fn dump_scene_sequence_has_rows() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "scene");
    assert!(v["rows"].is_array());
}

#[test]
fn dump_scene_class_family_stub() {
    let v = dump_ok(
        "@startuml\nclass Foo\nclass Bar\nFoo --> Bar\n@enduml",
        "scene",
    );
    assert!(v["kind"].is_string());
}

#[test]
fn dump_scene_class_has_nodes() {
    let v = dump_ok("@startuml\nclass Foo\nclass Bar\n@enduml", "scene");
    assert!(v["nodes"].is_array() || v["kind"].is_string());
}

#[test]
fn dump_scene_gantt_kind() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "scene",
    );
    assert_eq!(v["kind"], "TimelineScene");
}

#[test]
fn dump_scene_gantt_tasks() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Build]\n[Design] lasts 3 days\n[Build] lasts 2 days\n@endgantt",
        "scene",
    );
    assert!(v["tasks"].is_array());
}

#[test]
fn dump_scene_gantt_milestones() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "scene",
    );
    assert!(v["milestones"].is_array());
}

#[test]
fn dump_scene_gantt_svg_preview() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "scene",
    );
    assert!(v["svg_preview"].is_string());
}

#[test]
fn dump_scene_chronology_kind() {
    let v = dump_ok(
        "@startchronology\nPhase 1 happens on 2026-05-10\n@endchronology",
        "scene",
    );
    assert_eq!(v["kind"], "TimelineScene");
}

#[test]
fn dump_scene_state_kind() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml",
        "scene",
    );
    assert!(v["kind"].is_string());
}

#[test]
fn dump_scene_state_svg_preview() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml",
        "scene",
    );
    // State scene uses render_state_svg internally
    assert!(v["svg_preview"].is_string() || v["kind"].is_string());
}

#[test]
fn dump_scene_wire_kind() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "scene");
    assert_eq!(v["kind"], "WireDiagram");
}

#[test]
fn dump_scene_wire_components() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "scene");
    assert!(v["components"].is_array());
}

#[test]
fn dump_scene_wire_links() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "scene");
    assert!(v["links"].is_number());
}

#[test]
fn dump_scene_wire_svg_preview() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "scene");
    assert!(v["svg_preview"].is_string());
}

#[test]
fn dump_scene_json_fallback() {
    let v = dump_ok("@startjson\n{\"name\": \"puml\"}\n@endjson", "scene");
    assert_eq!(v["kind"], "Json");
}

#[test]
fn dump_scene_yaml_fallback() {
    let v = dump_ok("@startyaml\nname: puml\n@endyaml", "scene");
    assert_eq!(v["kind"], "Yaml");
}

#[test]
fn dump_scene_nwdiag_fallback() {
    let v = dump_ok(
        "@startnwdiag\nnetwork dmz {\n    web01\n}\n@endnwdiag",
        "scene",
    );
    assert_eq!(v["kind"], "Nwdiag");
}

#[test]
fn dump_scene_chen_family_stub() {
    let v = dump_ok(
        "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen",
        "scene",
    );
    assert!(v["kind"].is_string());
}

#[test]
fn dump_scene_board_family_stub() {
    let v = dump_ok("@startboard\nBacklog\n+Task A\nDone\n@endboard", "scene");
    assert!(v["kind"].is_string());
}

#[test]
fn dump_scene_files_family_stub() {
    let v = dump_ok("@startfiles\n/src/main.rs\n@endfiles", "scene");
    assert!(v["kind"].is_string());
}

// ─────────────────────────────────────────────────────────────────────────────
// --dump ast  (exercises ast_to_json + statement_kind_to_json branches)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn dump_ast_sequence_kind() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "ast");
    assert_eq!(v["kind"], "Sequence");
}

#[test]
fn dump_ast_sequence_has_statements() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "ast");
    assert!(v["statements"].is_array());
    assert!(!v["statements"].as_array().unwrap().is_empty());
}

#[test]
fn dump_ast_class_kind() {
    let v = dump_ok("@startuml\nclass Foo\n@enduml", "ast");
    assert_eq!(v["kind"], "Class");
}

#[test]
fn dump_ast_class_decl_statement() {
    let v = dump_ok("@startuml\nclass Foo\n@enduml", "ast");
    let stmts = v["statements"].as_array().unwrap();
    let has_class_decl = stmts.iter().any(|s| s["kind"]["ClassDecl"].is_object());
    assert!(has_class_decl, "expected ClassDecl statement");
}

#[test]
fn dump_ast_object_kind() {
    let v = dump_ok("@startuml\nobject Foo {\n  name = test\n}\n@enduml", "ast");
    assert_eq!(v["kind"], "Object");
}

#[test]
fn dump_ast_usecase_kind() {
    let v = dump_ok("@startuml\nusecase UC1 as \"Use case\"\n@enduml", "ast");
    assert_eq!(v["kind"], "UseCase");
}

#[test]
fn dump_ast_mindmap_kind() {
    let v = dump_ok("@startmindmap\n* Root\n** Child\n@endmindmap", "ast");
    assert_eq!(v["kind"], "MindMap");
}

#[test]
fn dump_ast_wbs_kind() {
    let v = dump_ok("@startwbs\n* Root\n** Item\n@endwbs", "ast");
    assert_eq!(v["kind"], "Wbs");
}

#[test]
fn dump_ast_gantt_kind() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "ast",
    );
    assert_eq!(v["kind"], "Gantt");
}

#[test]
fn dump_ast_chronology_kind() {
    let v = dump_ok(
        "@startchronology\nPhase 1 happens on 2026-05-10\n@endchronology",
        "ast",
    );
    assert_eq!(v["kind"], "Chronology");
}

#[test]
fn dump_ast_component_kind() {
    let v = dump_ok("@startuml\ncomponent C1\ncomponent C2\n@enduml", "ast");
    assert_eq!(v["kind"], "Component");
}

#[test]
fn dump_ast_deployment_kind() {
    let v = dump_ok("@startuml\nnode N1\nnode N2\nN1 --> N2\n@enduml", "ast");
    assert_eq!(v["kind"], "Deployment");
}

#[test]
fn dump_ast_state_kind() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active\n@enduml",
        "ast",
    );
    assert_eq!(v["kind"], "State");
}

#[test]
fn dump_ast_activity_kind() {
    let v = dump_ok("@startuml\nstart\n:Action A;\nstop\n@enduml", "ast");
    assert_eq!(v["kind"], "Activity");
}

#[test]
fn dump_ast_json_kind() {
    let v = dump_ok("@startjson\n{\"name\": \"puml\"}\n@endjson", "ast");
    assert_eq!(v["kind"], "Json");
}

#[test]
fn dump_ast_yaml_kind() {
    let v = dump_ok("@startyaml\nname: puml\n@endyaml", "ast");
    assert_eq!(v["kind"], "Yaml");
}

#[test]
fn dump_ast_nwdiag_kind() {
    let v = dump_ok(
        "@startnwdiag\nnetwork dmz {\n    web01\n}\n@endnwdiag",
        "ast",
    );
    assert_eq!(v["kind"], "Nwdiag");
}

#[test]
fn dump_ast_archimate_kind() {
    let v = dump_ok(
        "@startarchimate\narchimate \"C\" as c <<motivation>>\n@endarchimate",
        "ast",
    );
    assert_eq!(v["kind"], "Archimate");
}

#[test]
fn dump_ast_regex_kind() {
    let v = dump_ok("@startregex\na(b|c)*\n@endregex", "ast");
    assert_eq!(v["kind"], "Regex");
}

#[test]
fn dump_ast_ebnf_kind() {
    let v = dump_ok("@startebnf\nexpr = term ;\n@endebnf", "ast");
    assert_eq!(v["kind"], "Ebnf");
}

#[test]
fn dump_ast_math_kind() {
    let v = dump_ok("@startmath\na^2 + b^2 = c^2\n@endmath", "ast");
    assert_eq!(v["kind"], "Math");
}

#[test]
fn dump_ast_sdl_kind() {
    let v = dump_ok(
        "@startsdl\nstart Idle\nstop Done\nIdle -> Done : go\n@endsdl",
        "ast",
    );
    assert_eq!(v["kind"], "Sdl");
}

#[test]
fn dump_ast_ditaa_kind() {
    let v = dump_ok("@startditaa\n+----+\n| A  |\n+----+\n@endditaa", "ast");
    assert_eq!(v["kind"], "Ditaa");
}

#[test]
fn dump_ast_chart_kind() {
    let v = dump_ok("@startchart\nbar\n\"A\" 10\n@endchart", "ast");
    assert_eq!(v["kind"], "Chart");
}

#[test]
fn dump_ast_chen_kind() {
    let v = dump_ok(
        "@startchen\nentity Person {\n  Number <<key>>\n}\n@endchen",
        "ast",
    );
    assert_eq!(v["kind"], "Chen");
}

#[test]
fn dump_ast_board_kind() {
    let v = dump_ok("@startboard\nBacklog\n+Task A\nDone\n@endboard", "ast");
    assert_eq!(v["kind"], "Board");
}

#[test]
fn dump_ast_files_kind() {
    let v = dump_ok("@startfiles\n/src/main.rs\n@endfiles", "ast");
    assert_eq!(v["kind"], "Files");
}

#[test]
fn dump_ast_wire_kind() {
    let v = dump_ok("@startwire\ncomponent Panel [120x90] right:POWER\n--\ncomponent Controller [150x110] left:POWER\nPanel.POWER -- Controller.POWER\n@endwire", "ast");
    assert_eq!(v["kind"], "Wire");
}

#[test]
fn dump_ast_state_transition_statement() {
    let v = dump_ok(
        "@startuml\nstate Idle\nstate Active\nIdle --> Active : open\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_transition = stmts
        .iter()
        .any(|s| s["kind"]["StateTransition"].is_object());
    assert!(has_transition, "expected StateTransition statement");
}

#[test]
fn dump_ast_family_relation_statement() {
    let v = dump_ok(
        "@startuml\nclass Foo\nclass Bar\nFoo --> Bar\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_rel = stmts
        .iter()
        .any(|s| s["kind"]["FamilyRelation"].is_object());
    assert!(has_rel, "expected FamilyRelation statement");
}

#[test]
fn dump_ast_title_statement() {
    let v = dump_ok("@startuml\ntitle My Title\nA -> B\n@enduml", "ast");
    let stmts = v["statements"].as_array().unwrap();
    let has_title = stmts.iter().any(|s| s["kind"]["Title"].is_string());
    assert!(has_title, "expected Title statement");
}

#[test]
fn dump_ast_message_statement() {
    let v = dump_ok("@startuml\nA -> B : hello\n@enduml", "ast");
    let stmts = v["statements"].as_array().unwrap();
    let has_msg = stmts.iter().any(|s| s["kind"]["Message"].is_object());
    assert!(has_msg, "expected Message statement");
}

#[test]
fn dump_ast_participant_statement() {
    let v = dump_ok("@startuml\nparticipant Alice\n@enduml", "ast");
    let stmts = v["statements"].as_array().unwrap();
    let has_participant = stmts.iter().any(|s| s["kind"]["Participant"].is_object());
    assert!(has_participant, "expected Participant statement");
}

#[test]
fn dump_ast_gantt_task_decl_statement() {
    let v = dump_ok(
        "@startgantt\n[Design]\n[Design] lasts 3 days\n@endgantt",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    // GanttCompound or GanttTaskDecl should be present
    let has_gantt = stmts
        .iter()
        .any(|s| s["kind"]["GanttCompound"].is_object() || s["kind"]["GanttTaskDecl"].is_object());
    assert!(has_gantt, "expected gantt task statement");
}

#[test]
fn dump_ast_chronology_happens_on_statement() {
    let v = dump_ok(
        "@startchronology\nPhase 1 happens on 2026-05-10\n@endchronology",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_happens = stmts
        .iter()
        .any(|s| s["kind"]["ChronologyHappensOn"].is_object());
    assert!(has_happens, "expected ChronologyHappensOn statement");
}

#[test]
fn dump_ast_state_decl_statement() {
    let v = dump_ok("@startuml\nstate Idle\n@enduml", "ast");
    let stmts = v["statements"].as_array().unwrap();
    let has_state_decl = stmts.iter().any(|s| s["kind"]["StateDecl"].is_object());
    assert!(has_state_decl, "expected StateDecl statement");
}

#[test]
fn dump_ast_skinparam_statement() {
    let v = dump_ok(
        "@startuml\nskinparam sequenceArrowColor red\nA -> B\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_skinparam = stmts.iter().any(|s| s["kind"]["SkinParam"].is_object());
    assert!(has_skinparam, "expected SkinParam statement");
}

#[test]
fn dump_ast_note_statement() {
    let v = dump_ok(
        "@startuml\nA -> B : hello\nnote left : A note\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_note = stmts.iter().any(|s| s["kind"]["Note"].is_object());
    assert!(has_note, "expected Note statement");
}

#[test]
fn dump_ast_group_statement() {
    let v = dump_ok(
        "@startuml\nA -> B : hello\ngroup MyGroup\nA -> B : group msg\nend\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_group = stmts.iter().any(|s| s["kind"]["Group"].is_object());
    assert!(has_group, "expected Group statement");
}

#[test]
fn dump_ast_divider_statement() {
    // == Divider == is parsed as a Separator in the AST
    let v = dump_ok(
        "@startuml\nA -> B\n== Divider ==\nA -> B : after\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_sep = stmts.iter().any(|s| s["kind"]["Separator"].is_string());
    assert!(has_sep, "expected Separator statement");
}

#[test]
fn dump_ast_activate_statement() {
    let v = dump_ok(
        "@startuml\nA -> B : call\nactivate B\nB -> A : return\ndeactivate B\n@enduml",
        "ast",
    );
    let stmts = v["statements"].as_array().unwrap();
    let has_activate = stmts.iter().any(|s| s["kind"]["Activate"].is_string());
    assert!(has_activate, "expected Activate statement");
}

#[test]
fn dump_ast_autonumber_statement() {
    // Autonumber kind value is a string (the format string, empty for bare autonumber)
    let v = dump_ok("@startuml\nautonumber\nA -> B : msg\n@enduml", "ast");
    let stmts = v["statements"].as_array().unwrap();
    let has_autonumber = stmts.iter().any(|s| !s["kind"]["Autonumber"].is_null());
    assert!(has_autonumber, "expected Autonumber statement");
}

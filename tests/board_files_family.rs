use puml::model::NormalizedDocument;
use puml::{
    extract_metadata, normalize_family, parse, render_source_to_svg, render_source_to_text,
    DiagramFamily, TextOutputMode,
};

#[test]
fn board_family_parses_normalizes_renders_and_reports_metadata() {
    let source = r#"@startboard
title Sprint board
Backlog
+User Task 1 #release1
++Story 1
Doing
+Renderer scene contract
Done
+Guardrail shipped #ci
@endboard
"#;

    let document = parse(source).expect("parse board");
    assert_eq!(document.kind, puml::ast::DiagramKind::Board);
    let model = normalize_family(document.clone()).expect("normalize board");
    let NormalizedDocument::Board(board) = &model else {
        panic!("expected board model");
    };
    assert_eq!(board.columns.len(), 3);
    assert_eq!(board.columns[0].cards.len(), 2);
    assert!(board.warnings.is_empty());

    let svg = render_source_to_svg(source).expect("render board");
    assert!(svg.contains("class=\"board-column\""));
    assert!(svg.contains("User Task 1"));

    let text = render_source_to_text(source, TextOutputMode::Txt).expect("board text");
    assert!(text.contains("board"));
    assert!(text.contains("column Backlog"));

    let metadata = extract_metadata(&document, &model);
    assert_eq!(metadata.family, DiagramFamily::Board.as_str());
    assert_eq!(metadata.counts["columns"], 3);
    assert_eq!(metadata.counts["cards"], 4);
}

#[test]
fn board_family_warns_on_depth_beyond_first_supported_slice() {
    let source = r#"@startboard
Activity
+++++Too deep
@endboard
"#;

    let document = parse(source).expect("parse board");
    let model = normalize_family(document).expect("normalize board");
    let NormalizedDocument::Board(board) = model else {
        panic!("expected board model");
    };
    assert_eq!(board.warnings.len(), 1);
    assert!(board.warnings[0].message.contains("W_BOARD_DEPTH_LIMIT"));
    assert_eq!(board.columns[0].cards[0].depth, 4);
}

#[test]
fn files_family_merges_paths_preserves_order_and_attaches_notes() {
    let source = r#"@startfiles
title Repo files
<note>
top note
</note>
/.github/
/src/example.py
/tests/example_test.py
/src/example1.py
<note>
source sibling note
</note>
/README.md
@endfiles
"#;

    let document = parse(source).expect("parse files");
    assert_eq!(document.kind, puml::ast::DiagramKind::Files);
    let model = normalize_family(document.clone()).expect("normalize files");
    let NormalizedDocument::Files(files) = &model else {
        panic!("expected files model");
    };
    assert_eq!(files.title.as_deref(), Some("Repo files"));
    assert_eq!(files.top_notes, vec!["top note"]);
    assert_eq!(files.roots[0].name, ".github");
    assert_eq!(files.roots[1].name, "src");
    assert_eq!(files.roots[1].children.len(), 2);
    assert_eq!(
        files.roots[1].children[1].notes,
        vec!["source sibling note"]
    );

    let svg = render_source_to_svg(source).expect("render files");
    assert!(svg.contains("class=\"files-entry\""));
    assert!(svg.contains("data-files-path=\"/src/example1.py\""));

    let text = render_source_to_text(source, TextOutputMode::Txt).expect("files text");
    assert!(text.contains("files"));
    assert!(text.contains("dir src"));
    assert!(text.contains("file example.py"));

    let metadata = extract_metadata(&document, &model);
    assert_eq!(metadata.family, DiagramFamily::Files.as_str());
    assert_eq!(metadata.counts["roots"], 4);
    assert_eq!(metadata.counts["notes"], 2);
}

#[test]
fn files_family_warns_for_unsupported_lines_and_deep_paths() {
    let source = r#"@startfiles
not/a/path
/a/b/c/d/e/f/g/h/i.txt
@endfiles
"#;

    let document = parse(source).expect("parse files");
    let model = normalize_family(document).expect("normalize files");
    let NormalizedDocument::Files(files) = model else {
        panic!("expected files model");
    };
    let messages = files
        .warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>();
    assert!(messages
        .iter()
        .any(|message| message.contains("W_FILES_UNSUPPORTED_LINE")));
    assert!(messages
        .iter()
        .any(|message| message.contains("W_FILES_DEPTH_LIMIT")));
}

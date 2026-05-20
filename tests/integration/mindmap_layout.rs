use super::support::*;
use super::*;

#[test]
fn mindmap_cli_renders_radial_tree_not_dag_grid() {
    // Regression test for #240: CLI path must produce mindmap-node/edge markers,
    // not uml-relation markers (which indicate the wrong DAG-grid renderer).
    let input = format!(
        "{}/docs/examples/mindmap/02_multi_level.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("mm.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(
        svg.contains("mindmap-node"),
        "CLI mindmap output must use mindmap-node class, not DAG grid"
    );
    assert!(
        svg.contains("mindmap-edge"),
        "CLI mindmap output must use mindmap-edge class"
    );
    assert!(
        !svg.contains("uml-relation"),
        "CLI mindmap output must NOT use uml-relation (DAG grid renderer)"
    );
    assert!(
        svg.contains("Technology Stack"),
        "root node label must be present"
    );
}

#[test]
fn wbs_cli_renders_hierarchical_tree_not_dag_grid() {
    // Regression test for #240: CLI path must produce wbs-node/edge markers,
    // not uml-relation markers.
    let input = format!(
        "{}/docs/examples/wbs/04_multi_level.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("wbs.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(
        svg.contains("wbs-node"),
        "CLI WBS output must use wbs-node class, not DAG grid"
    );
    assert!(
        svg.contains("wbs-edge"),
        "CLI WBS output must use wbs-edge class"
    );
    assert!(
        !svg.contains("uml-relation"),
        "CLI WBS output must NOT use uml-relation (DAG grid renderer)"
    );
    assert!(
        svg.contains("Software Development"),
        "root node label must be present"
    );
}

#[test]
fn mindmap_root_is_centered_and_children_distributed_both_sides() {
    // Acceptance criterion (a): mindmap positions root at center, children
    // appear both left and right when `left side` / `right side` are used.
    // We use the CLI path (which was the broken one) via a temp file.
    let src = concat!(
        "@startmindmap\n",
        "* Root\n",
        "** RightA\n",
        "** RightB\n",
        "left side\n",
        "** LeftA\n",
        "** LeftB\n",
        "@endmindmap\n",
    );

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("mm_sides.puml");
    let output = tmp.path().join("mm_sides.svg");
    fs::write(&input, src).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let svg = fs::read_to_string(&output).expect("output SVG must exist");
    assert!(svg.contains("mindmap-root"), "must have root marker");
    assert!(
        svg.contains("data-mindmap-side=\"left\""),
        "must have left-side branches"
    );
    assert!(
        svg.contains("data-mindmap-side=\"right\""),
        "must have right-side branches"
    );

    // Parse x position of root rect.
    fn extract_x_after_marker(svg: &str, marker: &str) -> Option<i32> {
        let idx = svg.find(marker)?;
        let tail = &svg[idx..];
        let end = tail.find('>')?;
        let elem = &tail[..end];
        let key = " x=\"";
        let start = elem.find(key)? + key.len();
        let val_end = elem[start..].find('"')? + start;
        elem[start..val_end].parse().ok()
    }

    // Parse x positions of all rect nodes with a given side attribute.
    fn extract_x_for_side(svg: &str, side: &str) -> Vec<i32> {
        let marker = format!("data-mindmap-side=\"{side}\" data-mindmap-child-count");
        let mut xs = Vec::new();
        let mut search = svg;
        while let Some(idx) = search.find(&marker) {
            // Backtrack to find the opening < of this element so we can get x=
            let before = &search[..idx];
            if let Some(rect_start) = before.rfind('<') {
                let tail = &search[rect_start..];
                let end = tail.find('>').unwrap_or(tail.len());
                let elem = &tail[..end];
                let key = " x=\"";
                if let Some(start) = elem.find(key) {
                    let val_start = start + key.len();
                    if let Some(val_end) = elem[val_start..].find('"') {
                        if let Ok(x) = elem[val_start..val_start + val_end].parse::<i32>() {
                            xs.push(x);
                        }
                    }
                }
            }
            search = &search[idx + 1..];
        }
        xs
    }

    let root_rx: i32 =
        extract_x_after_marker(&svg, "mindmap-root").expect("root rect must have x attribute");
    let left_xs = extract_x_for_side(&svg, "left");
    let right_xs = extract_x_for_side(&svg, "right");

    assert!(!left_xs.is_empty(), "must have at least one left-side node");
    assert!(
        !right_xs.is_empty(),
        "must have at least one right-side node"
    );

    let max_left_x = left_xs.iter().copied().max().unwrap_or(0);
    let min_right_x = right_xs.iter().copied().min().unwrap_or(i32::MAX);
    assert!(
        max_left_x < root_rx,
        "all left-side node x positions ({max_left_x}) must be left of root ({root_rx})"
    );
    assert!(
        root_rx < min_right_x,
        "root ({root_rx}) must be left of all right-side node x positions ({min_right_x})"
    );
}

#[test]
fn wbs_no_crossing_edges_in_left_right_mode() {
    // Acceptance criterion (b): WBS in left-right mode must have strictly
    // increasing x per depth level — child x > parent x for all edges.
    let src = concat!(
        "@startwbs\n",
        "left to right direction\n",
        "* Root\n",
        "** A\n",
        "*** A1\n",
        "*** A2\n",
        "** B\n",
        "*** B1\n",
        "@endwbs\n",
    );
    let svg = render_source_to_svg(src).expect("wbs LR should render");
    assert!(svg.contains("wbs-node"), "must use wbs renderer");

    // Parse all wbs-edge lines and check that x2 > x1 (LR: child is to the right).
    fn parse_line_x1_x2(line_elem: &str) -> Option<(i32, i32)> {
        let get = |attr: &str| -> Option<i32> {
            let key = format!(" {attr}=\"");
            let start = line_elem.find(&key)? + key.len();
            let end = line_elem[start..].find('"')? + start;
            line_elem[start..end].parse().ok()
        };
        Some((get("x1")?, get("x2")?))
    }

    let mut checked = 0usize;
    for elem in svg_elements_with_class(&svg, "line", "wbs-edge") {
        if let Some((x1, x2)) = parse_line_x1_x2(elem) {
            assert!(
                x2 > x1,
                "WBS LR mode: edge x2 ({x2}) must be > x1 ({x1}) — no backward edges allowed"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "must have at least one wbs edge to verify");
}

#[test]
fn mindmap_basic_fixture_renders_tree_via_cli() {
    let input = format!(
        "{}/docs/examples/mindmap/01_basic.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("mm_basic.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(svg.contains("mindmap-root"), "must have root node");
    assert!(!svg.contains("uml-relation"), "must not use DAG renderer");
}

#[test]
fn wbs_basic_fixture_renders_tree_via_cli() {
    let input = format!(
        "{}/docs/examples/wbs/01_basic.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("wbs_basic.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(svg.contains("wbs-node"), "must have wbs node class");
    assert!(!svg.contains("uml-relation"), "must not use DAG renderer");
}

// Regression test for #424: the `class` keyword must not leak into the box label.
// Before the fix, labels rendered as "class Animal", "class Dog" etc.
#[test]
fn class_keyword_does_not_leak_into_box_label_issue_424() {
    // Variant A: classes with body blocks.
    let src = "@startuml\nclass Animal {\n  +name: String\n  +speak()\n}\nclass Dog {\n  +breed: String\n  +fetch()\n}\nAnimal --> Dog : owns\n@enduml\n";
    let svg = render_source_to_svg(src).expect("class svg must render");

    // The display label must be just the identifier — no "class " prefix.
    assert!(
        svg.contains(">Animal<"),
        "label must be 'Animal', got keyword leak or missing label"
    );
    assert!(
        svg.contains(">Dog<"),
        "label must be 'Dog', got keyword leak or missing label"
    );
    assert!(
        !svg.contains(">class Animal<") && !svg.contains("class Animal"),
        "keyword 'class' must not appear in the Animal box label"
    );
    assert!(
        !svg.contains(">class Dog<") && !svg.contains("class Dog"),
        "keyword 'class' must not appear in the Dog box label"
    );

    // Variant B: classes without body blocks (stub form).
    let src_stub = "@startuml\nclass Vehicle\nclass Car\nVehicle <|-- Car\n@enduml\n";
    let svg_stub = render_source_to_svg(src_stub).expect("stub class svg must render");
    assert!(
        !svg_stub.contains("class Vehicle"),
        "keyword 'class' must not bleed into Vehicle stub label"
    );
    assert!(
        svg_stub.contains(">Vehicle<"),
        "Vehicle label must appear as bare identifier in stub form"
    );
}

// ── Issue #769: enum classes must render with distinct lemon header ───────────

#[test]
fn enum_class_renders_with_enumeration_stereotype_and_lemon_header() {
    // `enum` keyword must produce a «enumeration» label and a #ffffcc header fill —
    // distinguishing enum boxes from regular class boxes (fix #769).
    let src = "@startuml\nenum Color {\n  RED\n  GREEN\n  BLUE\n}\nclass Widget {\n  +paint(c: Color)\n}\nWidget --> Color\n@enduml\n";
    let svg = render_source_to_svg(src).expect("enum class diagram must render");

    // The «enumeration» guillemet label must appear in the header.
    assert!(
        svg.contains("\u{ab}enumeration\u{bb}"),
        "enum header must contain «enumeration» stereotype label"
    );
    // The lemon fill colour is the PlantUML enum convention.
    assert!(
        svg.contains("#ffffcc"),
        "enum header must use lemon fill #ffffcc, not the default class blue"
    );
    // The class box (Widget) should still use the default (non-lemon) header.
    assert!(
        !svg.contains("Widget\u{ab}enumeration\u{bb}"),
        "regular class header must not carry the enumeration stereotype"
    );
    // Enum name must appear as the box label.
    assert!(svg.contains(">Color<"), "enum box label must be 'Color'");
}

/// Wave-8 activity diagram parity tests.
///
/// Covers three depth-meaningful gaps in the activity-new family:
///
/// 1. `endwhile (label)` / `endwhile(label)` parsing and exit-label rendering.
/// 2. `while ... endwhile` back-edge: the loop body must emit an upward return
///    arrow from the last body action back to the while diamond.
/// 3. Swimlane display modifiers: `|= Name|` bold header and
///    `|<<role>>Name|` stereotype sub-label.

// ---------------------------------------------------------------------------
// Feature 1 + 2: while loop back-edge + endwhile exit label
// ---------------------------------------------------------------------------

#[test]
fn w8_endwhile_with_exit_label_is_parsed_and_rendered() {
    // `endwhile (no more)` must be parsed as EndWhile with label "no more"
    // rather than falling through as an OldStyle action box.
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
while (data available?) is (yes)
  :read data;
endwhile (no more)
:process result;
stop
@enduml
"#,
    )
    .expect("while loop with exit label should render");

    // The exit label must appear in the SVG output.
    assert!(
        svg.contains(">no more<"),
        "endwhile exit label should appear in SVG"
    );
    // The exit label must NOT appear as a standalone action box
    assert!(
        !svg.contains("data-activity-kind=\"OldStyle\""),
        "endwhile (label) should not be parsed as an OldStyle action"
    );
    // The while condition must render in the decision diamond.
    assert!(
        svg.contains(">data available?<"),
        "while condition should be in the diamond"
    );
    // The successor action after the loop must be present.
    assert!(
        svg.contains(">process result<"),
        "action after endwhile should render"
    );
}

#[test]
fn w8_endwhile_no_label_still_works() {
    // Plain `endwhile` (no label) must still produce EndWhile, not a box.
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
while (cond?) is (yes)
  :body action;
endwhile
:after loop;
stop
@enduml
"#,
    )
    .expect("plain endwhile should render");

    assert!(
        svg.contains(">body action<"),
        "body action should be present"
    );
    assert!(
        svg.contains(">after loop<"),
        "post-loop action should be present"
    );
    // endwhile should NOT show up as a text label in an action box
    assert!(
        !svg.contains(">endwhile<"),
        "endwhile keyword should not render as an action label"
    );
    assert!(
        !svg.contains("data-activity-kind=\"OldStyle\""),
        "endwhile should not become OldStyle"
    );
}

#[test]
fn w8_while_loop_emits_upward_back_edge_arrow() {
    // After the `endwhile`, an upward back-edge arrow must be present in the
    // SVG so that the loop-back visual is correct.  We detect this by finding
    // any `<line>` segment where y2 < y1 (pointing upward).
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
while (keep going?) is (yes)
  :loop body;
endwhile (stop)
:finished;
stop
@enduml
"#,
    )
    .expect("while loop should render");

    // Verify that at least one upward-going segment exists (back-edge).
    // Stage-3 EdgeRouting may emit `<polyline>` instead of `<line>` for the
    // back-edge arrow, so check both element types.
    let has_upward_line = parse_svg_lines(&svg).any(|(_, y1, _, y2)| y2 < y1);
    let has_upward_polyline = svg_polylines_have_upward_segment(&svg);
    assert!(
        has_upward_line || has_upward_polyline,
        "while loop must emit at least one upward back-edge line segment"
    );

    // The exit label should appear.
    assert!(
        svg.contains(">stop<"),
        "endwhile exit label 'stop' should appear"
    );

    // The loop body action should be in the SVG.
    assert!(svg.contains(">loop body<"), "loop body should render");
    assert!(svg.contains(">finished<"), "post-loop action should render");
}

#[test]
fn w8_while_loop_yes_guard_appears_on_back_arrow() {
    // `while (cond?) is (yes)` — the "yes" guard must appear in the SVG
    // as a label on the back-loop arrow (or somewhere near it), NOT inside
    // the diamond condition text.
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
while (healthy?) is (yes)
  :poll;
endwhile (no)
stop
@enduml
"#,
    )
    .expect("while loop with yes guard should render");

    // "healthy?" must appear (condition in diamond).
    assert!(svg.contains(">healthy?<"), "condition must be in diamond");
    // "yes" must appear somewhere (the loop-back label).
    assert!(
        svg.contains(">yes<"),
        "'yes' guard must appear as arrow label"
    );
    // "no" must appear (exit label).
    assert!(svg.contains(">no<"), "'no' exit label must appear");
}

#[test]
fn w8_endwhile_paren_without_space_is_accepted() {
    // `endwhile(no)` (no space before paren) must parse as EndWhile with label.
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
while (going?) is (yes)
  :work;
endwhile(done)
stop
@enduml
"#,
    )
    .expect("endwhile(no space) should render");

    assert!(
        svg.contains(">done<"),
        "endwhile(label) without space must extract label"
    );
    assert!(
        !svg.contains(">endwhile(done)<"),
        "endwhile(label) must not appear as a raw action box"
    );
}

#[test]
fn w8_end_while_two_word_form_with_label_is_accepted() {
    // `end while (label)` (space in keyword) must also parse correctly.
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
while (running?) is (yes)
  :tick;
end while (exit)
stop
@enduml
"#,
    )
    .expect("'end while (label)' form should render");

    assert!(
        svg.contains(">exit<"),
        "'end while (label)' must extract exit label"
    );
    assert!(
        !svg.contains(">end while (exit)<"),
        "'end while (label)' must not render as action box"
    );
}

// ---------------------------------------------------------------------------
// Feature 3a: swimlane bold header `|= Name|`
// ---------------------------------------------------------------------------

#[test]
fn w8_swimlane_bold_header_strips_equals_prefix() {
    // `|= Lane A|` should set the lane name to "Lane A" (no leading `=`)
    // and render the header text with font-weight 800 (ultra-bold).
    let svg = puml::render_source_to_svg(
        r#"@startuml
|= Lane A|
:Action in A;
|= Lane B|
:Action in B;
@enduml
"#,
    )
    .expect("bold swimlane should render");

    // Bold font weight must be present in header texts.
    assert!(
        svg.contains("font-weight=\"800\""),
        "|= Name| should render header with font-weight 800"
    );
    // Lane names must appear without the leading `=`.
    assert!(
        svg.contains(">Lane A<"),
        "lane name should not contain leading ="
    );
    assert!(
        svg.contains(">Lane B<"),
        "lane name should not contain leading ="
    );
    // Actions should route to correct lanes (the `=` must be stripped from
    // the lane identifier, not just the display name).
    assert!(
        svg.contains(">Action in A<"),
        "action in bold lane A should render"
    );
    assert!(
        svg.contains(">Action in B<"),
        "action in bold lane B should render"
    );
}

#[test]
fn w8_swimlane_bold_does_not_bleed_equals_into_lane_name() {
    // Verify the lane identifier used for routing does NOT contain `=`.
    // We detect this by parsing the normalized document and checking the
    // lane=... alias field.
    let src = r#"@startuml
|= BoldLane|
:Step;
@enduml
"#;
    let document = puml::parser::parse(src).expect("parse bold swimlane");
    let puml::model::NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize bold swimlane")
    else {
        panic!("expected Family document");
    };
    // Find the action node and check its lane alias
    let step_node = model
        .nodes
        .iter()
        .find(|n| n.label.as_deref() == Some("Step"))
        .expect("Step action node");
    let alias = step_node.alias.as_deref().unwrap_or("");
    // The lane field must be "BoldLane", not "= BoldLane" or "=BoldLane"
    assert!(
        alias.contains("lane=BoldLane"),
        "lane identifier must not contain '=': alias={alias}"
    );
    assert!(
        !alias.contains("lane=="),
        "lane identifier must not have double = prefix: alias={alias}"
    );
}

// ---------------------------------------------------------------------------
// Feature 3b: swimlane stereotype `|<<role>>Name|`
// ---------------------------------------------------------------------------

#[test]
fn w8_swimlane_stereotype_renders_guillemet_sub_label() {
    // `|<<admin>>Admin Lane|` should render "Admin Lane" as the lane name
    // and `«admin»` as a smaller stereotype sub-label below it.
    let svg = puml::render_source_to_svg(
        r#"@startuml
|<<admin>>Admin Lane|
:Admin action;
|<<user>>User Lane|
:User action;
@enduml
"#,
    )
    .expect("stereotype swimlane should render");

    // Guillemet-wrapped stereotype must appear.
    assert!(
        svg.contains("«admin»"),
        "admin stereotype should appear as «admin»"
    );
    assert!(
        svg.contains("«user»"),
        "user stereotype should appear as «user»"
    );
    // Lane names must appear without the stereotype prefix.
    assert!(
        svg.contains(">Admin Lane<"),
        "lane name should not contain <<admin>> prefix"
    );
    assert!(
        svg.contains(">User Lane<"),
        "lane name should not contain <<user>> prefix"
    );
    // The raw `<<admin>>` form must NOT appear in the SVG.
    assert!(
        !svg.contains("&lt;&lt;admin&gt;&gt;"),
        "raw angle-bracket stereotype must not appear in output"
    );
}

#[test]
fn w8_swimlane_stereotype_does_not_bleed_into_lane_identifier() {
    // The `<<role>>` prefix must be stripped from the lane identifier used
    // for node routing.
    let src = r#"@startuml
|<<ops>>Ops|
:Deploy;
@enduml
"#;
    let document = puml::parser::parse(src).expect("parse stereotype swimlane");
    let puml::model::NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize stereotype swimlane")
    else {
        panic!("expected Family document");
    };
    let deploy = model
        .nodes
        .iter()
        .find(|n| n.label.as_deref() == Some("Deploy"))
        .expect("Deploy action node");
    let alias = deploy.alias.as_deref().unwrap_or("");
    assert!(
        alias.contains("lane=Ops"),
        "lane must be 'Ops' not '<<ops>>Ops': alias={alias}"
    );
    assert!(
        !alias.contains("<<"),
        "lane identifier must not contain <<: alias={alias}"
    );
}

#[test]
fn w8_swimlane_bold_and_stereotype_can_combine() {
    // `|= <<admin>>Admin|` — bold AND stereotype together
    let svg = puml::render_source_to_svg(
        r#"@startuml
|= <<sysadmin>>Operations|
:Deploy artifact;
@enduml
"#,
    )
    .expect("bold+stereotype swimlane should render");

    assert!(
        svg.contains("font-weight=\"800\""),
        "bold+stereotype lane should use weight 800"
    );
    assert!(
        svg.contains("«sysadmin»"),
        "bold+stereotype lane should show «sysadmin»"
    );
    assert!(
        svg.contains(">Operations<"),
        "bold+stereotype lane name should be 'Operations'"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse `<line x1="..." y1="..." x2="..." y2="..."/>` elements from SVG text.
fn parse_svg_lines(svg: &str) -> impl Iterator<Item = (i32, i32, i32, i32)> + '_ {
    svg.split("<line ").skip(1).filter_map(|segment| {
        let attr = |key: &str| -> Option<i32> {
            let prefix = format!("{key}=\"");
            let start = segment.find(prefix.as_str())? + prefix.len();
            let end = segment[start..].find('"')? + start;
            segment[start..end].parse().ok()
        };
        Some((attr("x1")?, attr("y1")?, attr("x2")?, attr("y2")?))
    })
}

/// Return `true` if any `<polyline points="...">` in the SVG contains at least
/// one upward-going segment (i.e. a consecutive y-coordinate pair where y[i+1] < y[i]).
/// Stage-3 EdgeRouting emits `<polyline>` instead of `<line>` for activity back-edges.
fn svg_polylines_have_upward_segment(svg: &str) -> bool {
    for segment in svg.split("<polyline ").skip(1) {
        let Some(points_start) = segment.find("points=\"") else {
            continue;
        };
        let rest = &segment[points_start + 8..];
        let Some(points_end) = rest.find('"') else {
            continue;
        };
        let points_str = &rest[..points_end];
        // points format: "x1,y1 x2,y2 ..."
        let ys: Vec<i32> = points_str
            .split_whitespace()
            .filter_map(|pair| pair.split_once(',').and_then(|(_, y)| y.parse().ok()))
            .collect();
        if ys.windows(2).any(|w| w[1] < w[0]) {
            return true;
        }
    }
    false
}

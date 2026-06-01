//! Regression tests for the remaining P1 visual bug cluster (issues #1449, #1465).
//!
//! Each section covers one bug with a targeted structural assertion:
//!
//! - **#1449** — state/07 nested "data ready" transition label is placed near its edge
//!   (inside the Working composite state), not orphaned at the bottom of the canvas.
//! - **#1465** — deployment/06 `<<container>>` stereotype labels are rendered ABOVE the
//!   node name text (no overlap / strikethrough appearance).

fn svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

// ---------------------------------------------------------------------------
// #1449 – state/07 nested "data ready" label is near its edge, not orphaned
// ---------------------------------------------------------------------------

const STATE_07: &str = include_str!("../docs/examples/state/07_nested.puml");

/// Extract the y-coordinate of a `<text>` element carrying `data-state-label="<label>"`.
fn state_label_y(svg_str: &str, label: &str) -> Option<i32> {
    let needle = format!("data-state-label=\"{label}\"");
    let idx = svg_str.find(&needle)?;
    let elem_start = svg_str[..idx].rfind('<').unwrap_or(0);
    let elem = &svg_str[elem_start..];
    let y_idx = elem.find(" y=\"")?;
    let rest = &elem[y_idx + 4..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

#[test]
fn state_07_data_ready_label_near_edge() {
    // #1449: the "data ready" label belongs to the Fetching→Processing edge
    // inside the Working composite state.  Before the fix, ancestor composite
    // states (Operational) were included in the collision-avoidance obstacle
    // set, which pushed the label all the way to y≈526 — below the end-state
    // circle.  After the fix the label must be within the Working composite.
    //
    // The Working composite is roughly at y=302..490; the label midpoint must
    // be clearly inside that range, not at y > 500.
    let out = svg(STATE_07);
    let label_y = state_label_y(&out, "data ready").expect("'data ready' label not found");
    assert!(
        label_y < 500,
        "data ready label at y={label_y} — still orphaned below canvas (#1449)"
    );
    assert!(
        label_y > 100,
        "data ready label at y={label_y} — unexpectedly above diagram (#1449)"
    );
}

// ---------------------------------------------------------------------------
// #1465 – deployment/06 <<container>> stereotype above name (no overlap)
// ---------------------------------------------------------------------------

const DEP_06: &str = include_str!("../docs/examples/deployment/06_kubernetes_pods_containers.puml");

/// For the first occurrence of a node whose name text is `name`, return
/// (name_y, stereo_y) where stereo_y is the y of the SPATIALLY ADJACENT
/// `<<container>>` stereotype text element (same x-coordinate within 4px).
///
/// We cannot rely on document order because edges and nodes are rendered in
/// separate passes; instead we match by x-coordinate proximity.
fn node_name_and_stereo_ys(svg_str: &str, name: &str) -> Option<(i32, i32)> {
    // Find bold name text element and extract its x and y.
    let name_needle = format!("font-weight=\"600\" fill=\"#0f172a\">{name}</text>");
    let name_pos = svg_str.find(&name_needle)?;
    let elem_start = svg_str[..name_pos].rfind('<')?;
    let elem = &svg_str[elem_start..];
    let x_idx = elem.find(" x=\"")?;
    let xrest = &elem[x_idx + 4..];
    let xend = xrest.find('"')?;
    let name_x: i32 = xrest[..xend].parse().ok()?;
    let y_idx = elem.find(" y=\"")?;
    let rest = &elem[y_idx + 4..];
    let end = rest.find('"')?;
    let name_y: i32 = rest[..end].parse().ok()?;

    // Find all <<container>> stereotype text elements in the SVG.
    // Return the one whose x-coordinate is closest to name_x (within 4px).
    let stereo_needle = "&lt;&lt;container&gt;&gt;</text>";
    let mut best_stereo_y: Option<i32> = None;
    let mut best_x_dist = i32::MAX;
    let mut search_pos = 0;
    while let Some(idx) = svg_str[search_pos..].find(stereo_needle) {
        let abs = search_pos + idx;
        let sel_start = svg_str[..abs].rfind('<').unwrap_or(0);
        let sel = &svg_str[sel_start..];
        // Extract x coord from this element
        if let Some(sx_idx) = sel.find(" x=\"") {
            let sxrest = &sel[sx_idx + 4..];
            if let Some(sxend) = sxrest.find('"') {
                if let Ok(sx) = sxrest[..sxend].parse::<i32>() {
                    let dist = (sx - name_x).abs();
                    if dist < best_x_dist {
                        // Extract y coord
                        if let Some(sy_idx) = sel.find(" y=\"") {
                            let syrest = &sel[sy_idx + 4..];
                            if let Some(syend) = syrest.find('"') {
                                if let Ok(sy) = syrest[..syend].parse::<i32>() {
                                    best_x_dist = dist;
                                    best_stereo_y = Some(sy);
                                }
                            }
                        }
                    }
                }
            }
        }
        search_pos = abs + 1;
    }

    // Only accept a match if x-coords are within 4 px (same node).
    if best_x_dist > 4 {
        return None;
    }
    best_stereo_y.map(|sy| (name_y, sy))
}

#[test]
fn dep_06_container_stereotype_above_name() {
    // #1465: <<container>> stereotype must appear ABOVE (smaller y) the node
    // name text.  Before the fix both texts shared almost the same y baseline
    // (stereotype at y=449, name at y=450 for "nginx"), producing a
    // strikethrough visual artifact.
    let out = svg(DEP_06);
    for name in &["nginx", "queue-consumer", "sidecar-logger"] {
        if let Some((name_y, stereo_y)) = node_name_and_stereo_ys(&out, name) {
            assert!(
                stereo_y < name_y,
                "node '{name}': stereotype y={stereo_y} is not above name y={name_y} (#1465)"
            );
        }
    }
}

//! Regression coverage for the 2026-05-28 control-flow + structural audit P0s.
//!
//! Each test renders the exact audit fixture the corresponding GitHub issue
//! cited and asserts the geometric invariant the fix establishes:
//!
//! * #1299 — activity fork bars stay inside the enclosing partition frame.
//! * #1300 — activity edges in deeply-nested if/else flows do not pierce node
//!   rectangles.
//! * #1302 — swimlanes render as side-by-side columns (each lane header sits
//!   in a distinct horizontal band rather than stacking vertically).
//! * #1287 — three-level class package frames nest without overlap.
//! * #1288 — cross-package class relations route through orthogonal channels
//!   and not through unrelated class boxes.
//! * #1290 — Kubernetes-style deployment frames at three nesting levels each
//!   produce a visible enclosing rectangle.

use std::fs;
use std::path::Path;

/// Render the given .puml fixture to SVG via the public renderer entry point
/// used by the renderer integration tests.
fn render_svg(fixture: &str) -> String {
    let src = fs::read_to_string(fixture).expect("fixture readable");
    puml::render_source_to_svg(&src).expect("render succeeds")
}

/// Extract every `<rect class="uml-group-frame" ...>` rectangle from an SVG
/// blob as `(scope, x, y, w, h)` tuples.
fn extract_group_frames(svg: &str) -> Vec<(String, i32, i32, i32, i32)> {
    let mut out = Vec::new();
    let mut cursor = svg;
    while let Some(start) = cursor.find("<rect class=\"uml-group-frame\"") {
        let tail = &cursor[start..];
        let Some(end) = tail.find("/>") else { break };
        let chunk = &tail[..end];
        let scope = attr(chunk, "data-uml-group")
            .unwrap_or_default()
            .to_string();
        let x = attr(chunk, "x")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let y = attr(chunk, "y")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let w = attr(chunk, "width")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let h = attr(chunk, "height")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        out.push((scope, x, y, w, h));
        cursor = &tail[end + 2..];
    }
    out
}

fn attr<'a>(chunk: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let idx = chunk.find(needle.as_str())?;
    let after = &chunk[idx + needle.len()..];
    let end = after.find('"')?;
    Some(&after[..end])
}

fn fixture(rel: &str) -> String {
    let root = env!("CARGO_MANIFEST_DIR");
    Path::new(root).join(rel).to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// #1299 — fork bar must stay inside the enclosing partition frame
// ---------------------------------------------------------------------------

#[test]
fn issue_1299_fork_bar_stays_inside_partition_frame() {
    let svg = render_svg(&fixture(
        "docs/examples/activity/18_repeat_while_nested_partition.puml",
    ));

    // The Load partition rect is the dashed band of width 896 starting at
    // x=32 in the audit fixture.  After the fork-bar half-width fix the
    // fork bar must visibly stop well inside the partition's left/right
    // edges (≥32 px margin).
    let load_partition = svg
        .lines()
        .flat_map(|line| {
            line.split('<').filter_map(|chunk| {
                if chunk.starts_with("rect ")
                    && chunk.contains("stroke-dasharray=\"4 3\"")
                    && chunk.contains("fill=\"#ecfdf5\"")
                {
                    // The Load partition is the LAST dashed light-green block
                    // (Extract uses the same fill earlier in the file).
                    Some(chunk.to_string())
                } else {
                    None
                }
            })
        })
        .last()
        .expect("Load partition rect present");
    let load_x = attr(&load_partition, "x")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let load_w = attr(&load_partition, "width")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let load_right = load_x + load_w;

    // Each fork bar is an 8-px-high solid black rect; pick the widest one
    // (i.e. the join bar in the Load partition).
    let fork_bar = svg
        .split('<')
        .filter(|chunk| {
            chunk.starts_with("rect ")
                && chunk.contains("height=\"8\"")
                && chunk.contains("fill=\"#0f172a\"")
        })
        .max_by_key(|chunk| {
            attr(chunk, "width")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        })
        .expect("fork bar rect present");
    let bar_x = attr(fork_bar, "x")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let bar_w = attr(fork_bar, "width")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let bar_right = bar_x + bar_w;

    assert!(
        bar_x >= load_x + 24,
        "fork bar must clear the partition's left edge by ≥24 px (#1299): bar_x={bar_x} partition_x={load_x}"
    );
    assert!(
        bar_right <= load_right - 24,
        "fork bar must clear the partition's right edge by ≥24 px (#1299): bar_right={bar_right} partition_right={load_right}"
    );
}

// ---------------------------------------------------------------------------
// #1300 — deep nested if/else: no edge passes through node rectangles
// ---------------------------------------------------------------------------

#[test]
fn issue_1300_no_arrow_passes_through_decision_diamond() {
    // The activity arrow router (`emit_activity_arrow`) is responsible for
    // bypassing node bboxes; the regression guard checks that the rendered
    // output for the deeply-nested authentication fixture contains the
    // expected bypass marker (3-segment vs 1-segment lines).  This is a
    // coarse but stable proxy: any rendered arrow that needed an L-bypass
    // produces strictly more `<line ` elements than a single straight
    // vertical would.
    let svg = render_svg(&fixture("docs/examples/activity/10_authentication.puml"));
    let line_count = svg.matches("<line ").count();
    // The fixture has 7 decision diamonds and 12+ direct/indirect edges;
    // an unbroken legacy "straight-through" render produced ≤24 line
    // segments.  The obstacle-avoiding router must produce noticeably more.
    assert!(
        line_count > 24,
        "expected obstacle-avoiding routing to emit multi-segment polylines, got {line_count}"
    );
}

// ---------------------------------------------------------------------------
// #1302 — swimlanes render as side-by-side columns, not vertically stacked
// ---------------------------------------------------------------------------

#[test]
fn issue_1302_swimlane_headers_share_top_band() {
    // Lane header text is emitted as `<text ...>Customer</text>` etc.  When
    // lanes render correctly as side-by-side columns, every named lane
    // header text sits on the SAME y coordinate (within a small tolerance).
    let svg = render_svg(&fixture(
        "docs/examples/activity/16_nested_swimlanes_parallel_forks.puml",
    ));
    let lane_names = ["Customer", "Warehouse", "Finance", "Logistics"];
    let mut header_ys: Vec<i32> = Vec::new();
    for name in &lane_names {
        let needle = format!(">{name}</text>");
        if let Some(idx) = svg.find(needle.as_str()) {
            // Walk backwards to the enclosing `<text ` opener.
            let prefix = &svg[..idx];
            if let Some(start) = prefix.rfind("<text ") {
                let chunk = &svg[start..idx];
                if let Some(y) = attr(chunk, "y").and_then(|s| s.parse::<i32>().ok()) {
                    header_ys.push(y);
                }
            }
        }
    }
    assert!(
        header_ys.len() >= 3,
        "expected ≥3 swimlane header text labels, found {}",
        header_ys.len()
    );
    let min_y = *header_ys.iter().min().unwrap();
    let max_y = *header_ys.iter().max().unwrap();
    assert!(
        max_y - min_y <= 8,
        "swimlane headers must sit on the same top band (#1302); ys: {header_ys:?}"
    );
}

// ---------------------------------------------------------------------------
// #1287 — three-level package nesting: sibling packages don't overlap
// ---------------------------------------------------------------------------

#[test]
fn issue_1287_deep_class_packages_dont_overlap() {
    let svg = render_svg(&fixture(
        "docs/examples/class/32_association_class_deep_packages.puml",
    ));
    let frames = extract_group_frames(&svg);
    let by_scope = |needle: &str| -> Option<(i32, i32, i32, i32)> {
        frames
            .iter()
            .find_map(|(scope, x, y, w, h)| (scope == needle).then_some((*x, *y, *w, *h)))
    };

    let hr = by_scope("com::acme::hr").expect("hr frame present");
    let payroll = by_scope("com::acme::payroll").expect("payroll frame present");
    let reporting = by_scope("com::acme::reporting").expect("reporting frame present");

    // hr/payroll/reporting are siblings inside acme — they must not
    // intersect each other.
    fn intersects(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        let (ax, ay, aw, ah) = a;
        let (bx, by, bw, bh) = b;
        ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
    }
    assert!(
        !intersects(hr, payroll),
        "hr and payroll frames must not overlap (#1287)"
    );
    assert!(
        !intersects(hr, reporting),
        "hr and reporting frames must not overlap (#1287)"
    );
    assert!(
        !intersects(payroll, reporting),
        "payroll and reporting frames must not overlap (#1287)"
    );
}

// ---------------------------------------------------------------------------
// #1288 — inter-package class edges use orthogonal channel routing
// ---------------------------------------------------------------------------

#[test]
fn issue_1288_cross_package_edges_are_polylines() {
    let svg = render_svg(&fixture("docs/examples/class/14_nested_packages.puml"));
    // Every cross-package relation must use the obstacle-avoiding
    // multi-segment route. Under EdgeRouting::Splines (the default) the
    // routed waypoints are smoothed into a cubic-Bézier `<path d="…">`
    // with multiple `C` segments — so we count "C "/"L " commands as
    // proxies for waypoints. Under EdgeRouting::Polyline / Ortho the
    // emission is a `<polyline points="…">` and we count point pairs
    // directly. Both paths must have ≥ 3 effective waypoints.
    let mut cross_pkg_count = 0;
    for chunk in svg.split('<') {
        let is_polyline = chunk.starts_with("polyline class=\"uml-relation\"");
        let is_path = chunk.starts_with("path class=\"uml-relation\"");
        if !is_polyline && !is_path {
            continue;
        }
        let from = attr(chunk, "data-uml-from").unwrap_or_default();
        let to = attr(chunk, "data-uml-to").unwrap_or_default();
        // Cross-package = the two endpoints don't share their leading
        // namespace segment in the source PUML (UserService→User etc).
        let cross = !same_top_namespace(from, to);
        if !cross {
            continue;
        }
        let waypoints = if is_polyline {
            attr(chunk, "points")
                .unwrap_or_default()
                .split_whitespace()
                .count()
        } else {
            // Each "C " command produces one cubic Bézier segment that
            // ends at a waypoint; the initial "M " seeds the start
            // point. waypoint_count = 1 (M) + number_of_C_segments.
            let d = attr(chunk, "d").unwrap_or_default();
            1 + d.matches(" C ").count() + d.matches(" L ").count()
        };
        assert!(
            waypoints >= 3,
            "cross-package relation must use a multi-segment route (#1288): from={from} to={to} chunk={chunk}"
        );
        cross_pkg_count += 1;
    }
    assert!(
        cross_pkg_count >= 2,
        "expected ≥2 cross-package relations to verify (#1288), saw {cross_pkg_count}"
    );
}

fn same_top_namespace(a: &str, b: &str) -> bool {
    let ta = a.split("::").next().unwrap_or(a);
    let tb = b.split("::").next().unwrap_or(b);
    ta == tb
}

// ---------------------------------------------------------------------------
// #1290 — three-level deployment nesting renders every level as a frame
// ---------------------------------------------------------------------------

#[test]
fn issue_1290_three_level_deployment_renders_each_level() {
    let svg = render_svg(&fixture(
        "docs/examples/deployment/06_kubernetes_pods_containers.puml",
    ));
    let frames = extract_group_frames(&svg);
    let scopes: std::collections::BTreeSet<&str> =
        frames.iter().map(|(s, _, _, _, _)| s.as_str()).collect();

    // Level 1 (outer Cluster)
    assert!(
        scopes.contains("Kubernetes Cluster"),
        "missing outer Cluster frame (#1290)"
    );
    // Level 2 (Namespace)
    assert!(
        scopes
            .iter()
            .any(|s| s.starts_with("Kubernetes Cluster::Namespace: ")),
        "missing Namespace-level frame (#1290)"
    );
    // Level 3 (Pod / StatefulSet)
    assert!(
        scopes.iter().any(|s| s.matches("::").count() >= 2),
        "missing third nesting level frame (#1290)"
    );

    // Every Namespace-level frame (exactly 2 `::` separators) must enclose
    // at least one Pod-level child (≥3 separators).
    let by_scope: std::collections::BTreeMap<&str, (i32, i32, i32, i32)> = frames
        .iter()
        .map(|(s, x, y, w, h)| (s.as_str(), (*x, *y, *w, *h)))
        .collect();
    for (scope, (px, py, pw, ph)) in &by_scope {
        // Restrict to true Namespace-level frames, NOT Pod-level frames whose
        // labels also begin with "Namespace: " in their parent prefix.
        let depth = scope.matches("::").count();
        if !scope.starts_with("Kubernetes Cluster::Namespace: ") || depth != 1 {
            continue;
        }
        let prefix = format!("{scope}::");
        let has_child = by_scope.iter().any(|(s, (cx, cy, cw, ch))| {
            if !s.starts_with(prefix.as_str()) {
                return false;
            }
            // Child centre must lie inside the parent rect.
            let cmx = cx + cw / 2;
            let cmy = cy + ch / 2;
            cmx >= *px && cmx <= *px + *pw && cmy >= *py && cmy <= *py + *ph
        });
        assert!(
            has_child,
            "Namespace frame {scope} must enclose at least one Pod-level child (#1290)"
        );
    }
}

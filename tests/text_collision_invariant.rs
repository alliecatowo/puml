//! Collision-free text invariant: discovery pass.
//!
//! Asserts that no `<text>` element's estimated bbox overlaps any node bbox
//! EXCEPT the node that the text belongs to.  This catches text-into-foreign-node
//! regressions introduced by density retunes, label-push logic, or layout changes.
//!
//! ## How it works
//!
//! For every fixture under `docs/examples/**/*.puml` and `docs/diagrams/**/*.puml`:
//!
//! 1. Render to SVG via `puml::render_source_to_svg`.
//! 2. Parse the SVG with `roxmltree`.
//! 3. Collect **node bboxes**: `<rect class="uml-node …" data-uml-id="…">` elements.
//! 4. Collect **text elements**: every `<text>` node with `x`/`y`/`font-size`
//!    attributes and visible text content.
//! 5. Estimate each text element's bbox: anchor `(x, y)` for baseline, using
//!    `font-size` and character count to approximate width.  A ±2 px tolerance
//!    covers anti-aliasing rounding.
//! 6. Determine **owner**: a text element "belongs to" a node when:
//!    - Its parent or an ancestor `<g>` has `data-uml-id` matching a node id, OR
//!    - Its `data-uml-label-role` is `"edge"` or `"edge-background"` (edge labels –
//!      ownership is the edge, not any node), OR
//!    - Position fallback: the text anchor point lies inside the node bbox.
//! 7. For each (text, node) pair where text is NOT owned by that node, assert
//!    their bboxes do not overlap (with ±2 px tolerance on each side).
//!
//! ## Expected outcome on first run
//!
//! This test is marked `#[ignore]` so CI is not hard-blocked while failures are
//! triaged.  Run it manually:
//!
//! ```sh
//! cargo test --release --test text_collision_invariant -- --ignored
//! ```
//!
//! FIXME(#1439): Remove `#[ignore]` once all surfaced violations are resolved.
//! Each violation below is a candidate for a follow-up P1 ticket.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
struct Bbox {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Bbox {
    fn right(self) -> f64 {
        self.x + self.w
    }
    fn bottom(self) -> f64 {
        self.y + self.h
    }

    /// True when this bbox overlaps `other`, with a symmetric tolerance applied
    /// to each edge (shrinking this bbox by `tol` on each side before testing).
    fn overlaps_with_tolerance(self, other: Bbox, tol: f64) -> bool {
        // Shrink self by `tol` on all four sides.
        let sx = self.x + tol;
        let sy = self.y + tol;
        let sr = self.right() - tol;
        let sb = self.bottom() - tol;
        // Degenerate after shrink – treat as no overlap.
        if sr <= sx || sb <= sy {
            return false;
        }
        // AABB intersection test.
        sx < other.right() && sr > other.x && sy < other.bottom() && sb > other.y
    }

    /// True when point (px, py) is inside this bbox (used for position fallback).
    fn contains_point(self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }
}

/// A node bbox extracted from the SVG.
#[derive(Debug, Clone)]
struct NodeBbox {
    id: String,
    bbox: Bbox,
    /// True for group frames (uml-group-frame), which have a large bbox that
    /// legitimately contains many child text elements.
    is_group_frame: bool,
}

/// How a `<text>` element's `x` attribute is interpreted relative to the glyph.
#[derive(Debug, Clone, Copy)]
enum TextAnchor {
    Start,  // x = left edge (default in SVG)
    Middle, // x = horizontal center
    End,    // x = right edge
}

/// A text element extracted from the SVG.
#[derive(Debug, Clone)]
struct TextElem {
    /// Anchor x (the SVG `x` attribute).
    ax: f64,
    /// Anchor y – this is the text baseline in SVG.
    ay: f64,
    /// Estimated width in pixels.
    est_w: f64,
    /// Estimated height in pixels (roughly font-size).
    est_h: f64,
    /// Visible text content.
    content: String,
    /// How `ax` is interpreted (start = left edge, middle = center, end = right edge).
    anchor: TextAnchor,
    /// Whether this text is an edge label (owned by an edge, not a node).
    is_edge_label: bool,
    /// The `data-uml-id` of the nearest ancestor element that has one, if any.
    ancestor_node_id: Option<String>,
}

impl TextElem {
    /// Estimated bbox.  SVG `y` is the text baseline; the visual box extends
    /// `est_h` upward from the baseline.  `ax` is adjusted for text-anchor.
    fn bbox(&self) -> Bbox {
        let x_left = match self.anchor {
            TextAnchor::Start => self.ax,
            TextAnchor::Middle => self.ax - self.est_w / 2.0,
            TextAnchor::End => self.ax - self.est_w,
        };
        Bbox {
            x: x_left,
            y: self.ay - self.est_h,
            w: self.est_w,
            h: self.est_h,
        }
    }
}

/// A single text-vs-node collision.
#[derive(Debug)]
struct Collision {
    fixture: String,
    text_content: String,
    text_bbox: Bbox,
    node_id: String,
    node_bbox: Bbox,
}

impl std::fmt::Display for Collision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] text {:?} bbox({:.0},{:.0},{:.0},{:.0}) \
             overlaps node {:?} bbox({:.0},{:.0},{:.0},{:.0})",
            self.fixture,
            self.text_content,
            self.text_bbox.x,
            self.text_bbox.y,
            self.text_bbox.w,
            self.text_bbox.h,
            self.node_id,
            self.node_bbox.x,
            self.node_bbox.y,
            self.node_bbox.w,
            self.node_bbox.h,
        )
    }
}

// ---------------------------------------------------------------------------
// SVG parsing helpers
// ---------------------------------------------------------------------------

fn parse_f64(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn has_class(class_attr: &str, needle: &str) -> bool {
    class_attr.split_whitespace().any(|c| c == needle)
}

/// Estimate text width from character count and font-size.
///
/// We use a constant factor of ~0.62 × font-size per character, which approximates
/// the monospace fonts used in PUML SVG output.  This intentionally over-estimates
/// slightly to catch real collisions without false negatives.
fn estimate_text_width(content: &str, font_size: f64) -> f64 {
    // Decode common HTML entities before measuring length.
    let decoded_len = content
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .chars()
        .count();
    (decoded_len as f64) * font_size * 0.62
}

/// Extract all node bboxes from the SVG.
///
/// Nodes: `<rect class="uml-node …" data-uml-id="…">`
/// Group frames: `<rect class="uml-group-frame" data-uml-group="…">`
fn extract_nodes(svg: &str) -> Vec<NodeBbox> {
    let doc = match roxmltree::Document::parse(svg) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut nodes = Vec::new();

    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "rect")
    {
        let class = node.attribute("class").unwrap_or("");
        let is_node = has_class(class, "uml-node");
        let is_group = has_class(class, "uml-group-frame");

        if !is_node && !is_group {
            continue;
        }

        // Extract id: data-uml-id (nodes) or data-uml-group (group frames).
        let id = if is_node {
            node.attribute("data-uml-id")
        } else {
            node.attribute("data-uml-group")
        };
        let Some(id) = id else { continue };

        let x = parse_f64(node.attribute("x").unwrap_or("0")).unwrap_or(0.0);
        let y = parse_f64(node.attribute("y").unwrap_or("0")).unwrap_or(0.0);
        let w = parse_f64(node.attribute("width").unwrap_or("0")).unwrap_or(0.0);
        let h = parse_f64(node.attribute("height").unwrap_or("0")).unwrap_or(0.0);

        nodes.push(NodeBbox {
            id: id.to_string(),
            bbox: Bbox { x, y, w, h },
            is_group_frame: is_group,
        });
    }

    nodes
}

/// Walk ancestors of `node` looking for the nearest `data-uml-id` attribute.
fn ancestor_uml_id<'a, 'input: 'a>(node: roxmltree::Node<'a, 'input>) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.is_element() {
            if let Some(id) = parent.attribute("data-uml-id") {
                return Some(id.to_string());
            }
        }
        current = parent.parent();
    }
    None
}

/// Extract all text elements from the SVG.
fn extract_texts(svg: &str) -> Vec<TextElem> {
    let doc = match roxmltree::Document::parse(svg) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut texts = Vec::new();

    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "text")
    {
        let ax = match node.attribute("x").and_then(parse_f64) {
            Some(v) => v,
            None => continue,
        };
        let ay = match node.attribute("y").and_then(parse_f64) {
            Some(v) => v,
            None => continue,
        };

        // Collect visible text content from text-node children only.
        //
        // roxmltree's `descendants()` yields BOTH element nodes (whose `.text()`
        // returns the first child text's content) AND the text nodes themselves —
        // causing a double-collection if we map `.text()` over all descendants.
        // Filter to only `is_text()` nodes to avoid duplicating the content.
        let content: String = node
            .descendants()
            .filter(|d| d.is_text())
            .filter_map(|d| d.text())
            .collect::<String>()
            .trim()
            .to_string();

        if content.is_empty() {
            continue;
        }

        let font_size: f64 = node
            .attribute("font-size")
            .and_then(parse_f64)
            .unwrap_or(12.0);

        let label_role = node.attribute("data-uml-label-role").unwrap_or("");
        let class = node.attribute("class").unwrap_or("");
        let is_edge_label = label_role == "edge"
            || label_role == "edge-background"
            || has_class(class, "uml-edge-label");

        let anchor = match node.attribute("text-anchor").unwrap_or("start") {
            "middle" => TextAnchor::Middle,
            "end" => TextAnchor::End,
            _ => TextAnchor::Start,
        };

        let ancestor_node_id = ancestor_uml_id(node);

        texts.push(TextElem {
            ax,
            ay,
            est_w: estimate_text_width(&content, font_size),
            est_h: font_size,
            content,
            anchor,
            is_edge_label,
            ancestor_node_id,
        });
    }

    texts
}

// ---------------------------------------------------------------------------
// Collision detection
// ---------------------------------------------------------------------------

const TOLERANCE_PX: f64 = 2.0;

/// Check one SVG for text-vs-node collisions.
///
/// Returns a list of collisions found.  An empty list means the invariant holds.
fn check_svg(svg: &str, fixture_name: &str) -> Vec<Collision> {
    let nodes = extract_nodes(svg);
    let texts = extract_texts(svg);

    let mut collisions = Vec::new();

    for text in &texts {
        let tbbox = text.bbox();

        if text.is_edge_label {
            // Edge labels should not enter any concrete node bbox.
            // Group frames are skipped: an edge routed inside a package legitimately
            // passes through the group frame bbox.
            for node in &nodes {
                if node.is_group_frame {
                    continue;
                }
                if tbbox.overlaps_with_tolerance(node.bbox, TOLERANCE_PX) {
                    collisions.push(Collision {
                        fixture: fixture_name.to_string(),
                        text_content: text.content.clone(),
                        text_bbox: tbbox,
                        node_id: node.id.clone(),
                        node_bbox: node.bbox,
                    });
                }
            }
            continue;
        }

        // Non-edge text: find the owner node.
        //
        // Ownership algorithm:
        // 1. If the text has an ancestor with data-uml-id, that node is the owner.
        // 2. Otherwise fall back to position: whichever (non-group) node's bbox
        //    contains the text anchor point.  If multiple match, pick the smallest.
        // 3. If no owner found, the text is "free" – check against all nodes.

        let owner_id: Option<String> = if let Some(ref id) = text.ancestor_node_id {
            Some(id.clone())
        } else {
            // Position-based fallback: smallest non-group node bbox that contains
            // the text anchor point (ax, ay).
            nodes
                .iter()
                .filter(|n| !n.is_group_frame && n.bbox.contains_point(text.ax, text.ay))
                .min_by(|a, b| {
                    let area_a = a.bbox.w * a.bbox.h;
                    let area_b = b.bbox.w * b.bbox.h;
                    area_a
                        .partial_cmp(&area_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|n| n.id.clone())
        };

        for node in &nodes {
            // Skip the owner.
            if owner_id.as_deref() == Some(&node.id) {
                continue;
            }

            // Group frames legitimately contain child text elements (node labels
            // belonging to children that live inside the frame).  Skip.
            if node.is_group_frame {
                continue;
            }

            if tbbox.overlaps_with_tolerance(node.bbox, TOLERANCE_PX) {
                collisions.push(Collision {
                    fixture: fixture_name.to_string(),
                    text_content: text.content.clone(),
                    text_bbox: tbbox,
                    node_id: node.id.clone(),
                    node_bbox: node.bbox,
                });
            }
        }
    }

    collisions
}

// ---------------------------------------------------------------------------
// Fixture collection
// ---------------------------------------------------------------------------

fn collect_puml_files(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return result;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            result.extend(collect_puml_files(&path));
        } else if path.extension().is_some_and(|e| e == "puml") {
            result.push(path);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Main invariant test
// ---------------------------------------------------------------------------

/// Discovery pass: run the text-collision invariant over every docs fixture.
///
/// FIXME(#1439): This test is `#[ignore]`d because the first run is expected to
/// surface real violations that need follow-up fix tickets.  Do not un-ignore
/// until all violations reported here are resolved or tracked as tickets.
///
/// To run manually:
/// ```sh
/// cargo test --release --test text_collision_invariant text_no_foreign_node_overlap -- --ignored 2>&1 | tee /tmp/collision-failures.txt
/// ```
#[test]
#[ignore = "FIXME(#1439): discovery pass; violations need follow-up fix tickets before this becomes a hard gate"]
fn text_no_foreign_node_overlap() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let examples_root = PathBuf::from(manifest_dir).join("docs/examples");
    let diagrams_root = PathBuf::from(manifest_dir).join("docs/diagrams");

    let mut all_fixtures: Vec<PathBuf> = Vec::new();
    all_fixtures.extend(collect_puml_files(&examples_root));
    all_fixtures.extend(collect_puml_files(&diagrams_root));

    assert!(
        !all_fixtures.is_empty(),
        "no .puml fixtures found under docs/examples or docs/diagrams"
    );

    let mut total = 0usize;
    let mut failed_fixtures = 0usize;
    let mut all_collisions: Vec<Collision> = Vec::new();
    let mut render_errors: Vec<String> = Vec::new();

    for fixture_path in &all_fixtures {
        total += 1;

        let source = match std::fs::read_to_string(fixture_path) {
            Ok(s) => s,
            Err(e) => {
                render_errors.push(format!("{}: read error: {e}", fixture_path.display()));
                continue;
            }
        };

        let svg = match puml::render_source_to_svg(&source) {
            Ok(s) => s,
            Err(e) => {
                // Some fixtures may legitimately not render (unsupported family,
                // etc.).  Record but do not fail the invariant test.
                render_errors.push(format!("{}: render error: {e:?}", fixture_path.display()));
                continue;
            }
        };

        let fixture_name = fixture_path
            .strip_prefix(manifest_dir)
            .unwrap_or(fixture_path)
            .display()
            .to_string();

        let collisions = check_svg(&svg, &fixture_name);
        if !collisions.is_empty() {
            failed_fixtures += 1;
            all_collisions.extend(collisions);
        }
    }

    // Print a summary visible in test output.
    let rendered_ok = total - render_errors.len();
    let pass = rendered_ok.saturating_sub(failed_fixtures);
    eprintln!(
        "\n=== Text-collision invariant discovery pass ===\n\
         Total fixtures : {total}\n\
         Rendered OK    : {rendered_ok}\n\
         Pass           : {pass}\n\
         Fail (collide) : {failed_fixtures}\n\
         Render errors  : {}\n\
         Total violations: {}",
        render_errors.len(),
        all_collisions.len(),
    );

    if !render_errors.is_empty() {
        eprintln!("\nRender errors (not collision failures):");
        for err in &render_errors {
            eprintln!("  {err}");
        }
    }

    if !all_collisions.is_empty() {
        eprintln!("\nAll violations ({} total):", all_collisions.len());
        for c in &all_collisions {
            eprintln!("  {c}");
        }
    }

    // Hard-fail with a clear summary.
    assert!(
        all_collisions.is_empty(),
        "\n{failed_fixtures} fixture(s) have text-vs-node collisions ({} total).\n\
         File follow-up tickets per FIXME(#1439).\n\n\
         First 20 violations:\n{}",
        all_collisions.len(),
        all_collisions
            .iter()
            .take(20)
            .map(|c| format!("  {c}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

// ---------------------------------------------------------------------------
// Unit tests for the bbox estimator and collision logic (always run, no #[ignore])
// ---------------------------------------------------------------------------

#[test]
fn bbox_overlap_basic() {
    let a = Bbox {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 50.0,
    };
    let b = Bbox {
        x: 50.0,
        y: 25.0,
        w: 100.0,
        h: 50.0,
    };
    assert!(
        a.overlaps_with_tolerance(b, 0.0),
        "overlapping boxes should collide"
    );
}

#[test]
fn bbox_no_overlap_adjacent() {
    let a = Bbox {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 50.0,
    };
    let b = Bbox {
        x: 100.0,
        y: 0.0,
        w: 100.0,
        h: 50.0,
    };
    assert!(
        !a.overlaps_with_tolerance(b, 0.0),
        "adjacent non-overlapping boxes should not collide"
    );
}

#[test]
fn bbox_tolerance_suppresses_near_miss() {
    // b starts 1px into a's right edge – within tolerance.
    let a = Bbox {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 50.0,
    };
    let b = Bbox {
        x: 99.0,
        y: 10.0,
        w: 50.0,
        h: 30.0,
    };
    // Without tolerance: 1px overlap → collide.
    assert!(a.overlaps_with_tolerance(b, 0.0));
    // With 2px tolerance applied to `a` (shrinks a's right edge by 2px): no collision.
    assert!(!a.overlaps_with_tolerance(b, 2.0));
}

#[test]
fn text_estimate_width_scales_with_content_and_font_size() {
    let w1 = estimate_text_width("Hello", 12.0);
    let w2 = estimate_text_width("Hello", 24.0);
    assert!(
        w2 > w1,
        "larger font-size should produce wider estimated bbox: {w1} vs {w2}"
    );
    let w3 = estimate_text_width("Hello World", 12.0);
    assert!(
        w3 > w1,
        "longer text should produce wider estimated bbox: {w1} vs {w3}"
    );
}

#[test]
fn check_svg_clean_diagram_no_collisions() {
    // A simple class diagram where text is clearly inside its own node.
    let source = r#"
@startuml
class Animal {
  + name: String
}
class Dog {
  + breed: String
}
Animal --> Dog
@enduml
"#;
    let svg = puml::render_source_to_svg(source).expect("render ok");
    let collisions = check_svg(&svg, "synthetic:basic_class");
    assert!(
        collisions.is_empty(),
        "simple class diagram should have no text collisions: {collisions:?}"
    );
}

#[test]
fn check_svg_does_not_flag_text_inside_own_node() {
    // Synthetic SVG: one node with its text inside the node bbox.
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 200 100">"#,
        r#"<rect class="uml-node uml-class" data-uml-id="MyClass" x="20" y="20" width="160" height="60"/>"#,
        // Text anchor (100, 55) lies inside the node bbox → position fallback assigns ownership.
        r#"<text x="100" y="55" text-anchor="middle" font-family="monospace" font-size="13">MyClass</text>"#,
        r#"</svg>"#
    );
    let collisions = check_svg(svg, "synthetic:inside_own_node");
    assert!(
        collisions.is_empty(),
        "text inside its own node should not be flagged: {collisions:?}"
    );
}

#[test]
fn check_svg_detects_text_inside_foreign_node() {
    // Synthetic SVG: two widely-separated nodes; text anchor sits in the gap
    // between them, but belongs to NodeA by position fallback (inside NodeA).
    // The estimated text bbox is wide enough to intrude into NodeB.
    //
    // NodeA: x=20..170   NodeB: x=300..450
    // Text anchor at x=140 (inside NodeA, owned by NodeA),
    // font-size=13, content="NodeA label" (12 chars → est_w ≈ 96px).
    // Text bbox: x_left = 140 − 48 = 92, x_right = 140 + 48 = 188 — still inside A.
    //
    // To produce a genuine intrusion: place the text anchor at x=160 (just inside
    // NodeA's right edge at x=170), wide content "NodeA long label text" (22 chars
    // → est_w ≈ 177 px).  Text bbox right edge: 160 + 88 = 248, which is inside
    // NodeB starting at x=200.
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="100" viewBox="0 0 500 100">"#,
        // NodeA at x=20..170
        r#"<rect class="uml-node uml-class" data-uml-id="NodeA" x="20" y="20" width="150" height="60"/>"#,
        // NodeB at x=200..350
        r#"<rect class="uml-node uml-class" data-uml-id="NodeB" x="200" y="20" width="150" height="60"/>"#,
        // Text anchor at x=160 (inside NodeA → owned by NodeA via position fallback).
        // At font-size=13 and 22 chars, est_w ≈ 13 * 0.62 * 22 ≈ 177px.
        // Estimated bbox: x_left = 160 − 88 = 72, x_right = 160 + 88 = 248.
        // x_right=248 > NodeB.x=200 → collision.
        r#"<text x="160" y="55" text-anchor="middle" font-family="monospace" font-size="13">NodeA long label text</text>"#,
        r#"</svg>"#
    );
    let collisions = check_svg(svg, "synthetic:foreign_node_intrusion");
    assert!(
        !collisions.is_empty(),
        "text bbox intruding into a foreign node should be flagged as a collision"
    );
    // The first collision must be against NodeB (the intruded-into node).
    let node_b_collision = collisions.iter().find(|c| c.node_id == "NodeB");
    assert!(
        node_b_collision.is_some(),
        "NodeB should be flagged as the colliding node; got: {:?}",
        collisions.iter().map(|c| &c.node_id).collect::<Vec<_>>()
    );
}

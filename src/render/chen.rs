/// Chen / EER entity-relationship diagram renderer.
///
/// Visual conventions:
///   - Entity         - rectangle (double-border for weak entities)
///   - Attribute      - oval; key attributes have underlined text
///   - Relationship   - diamond (double-border for identifying relationships)
///   - Lines          - straight line connecting shapes
///
/// Layout strategy (Wave-1 baseline):
///   - Entities placed in a grid (3 columns max), 200 px cell size.
///   - Relationships placed between each pair of participating entities
///     (centroid of participant bounding box).
///   - Attributes placed around the perimeter of their owner (entity or
///     relationship) in an evenly-spaced ring.
use super::*;
use crate::model::{ChenAttrKind, ChenDocument};

// ─── Layout constants ─────────────────────────────────────────────────────────

const ENTITY_W: f64 = 120.0;
const ENTITY_H: f64 = 44.0;
const ATTR_RX: f64 = 40.0; // horizontal radius of attribute oval
const ATTR_RY: f64 = 18.0; // vertical radius
const DIAMOND_HALF_W: f64 = 54.0;
const DIAMOND_HALF_H: f64 = 24.0;
const GRID_COLS: usize = 3;
const COL_SPACING: f64 = 240.0; // horizontal distance between entity centers
const ROW_SPACING: f64 = 200.0; // vertical distance between entity rows
const MARGIN: f64 = 120.0;
const ATTR_ORBIT_R: f64 = 70.0; // radius of attribute orbit around owner center

const FONT_ATTRS: &str = "font-family=\"Arial,Helvetica,sans-serif\" font-size=\"13\"";
const FONT_TITLE: &str = "font-family=\"Arial,Helvetica,sans-serif\" font-size=\"16\" font-weight=\"bold\"";

// Entity fill/stroke
const ENTITY_FILL: &str = "#dbeafe";
const ENTITY_STROKE: &str = "#2563eb";
const ENTITY_TEXT: &str = "#1e3a8a";
// Weak entity outline
const WEAK_STROKE: &str = "#4444cc";
// Relationship fill/stroke
const REL_FILL: &str = "#ede9fe";
const REL_STROKE: &str = "#7c3aed";
const REL_TEXT: &str = "#4c1d95";
// Attribute fill/stroke
const ATTR_FILL: &str = "#f9fafb";
const ATTR_STROKE: &str = "#374151";
const ATTR_TEXT: &str = "#374151";
// Line colors
const CONN_COLOR: &str = "#555555";
const ATTR_LINE_COLOR: &str = "#888888";

// ─── Layout types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Pt {
    x: f64,
    y: f64,
}

impl Pt {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Resolved position for an entity center.
struct EntityLayout {
    name: String,
    cx: f64,
    cy: f64,
    is_weak: bool,
    attr_positions: Vec<(Pt, String, ChenAttrKind)>, // (center, attr_name, kind)
}

/// Resolved position for a relationship diamond center.
struct RelLayout {
    name: String,
    cx: f64,
    cy: f64,
    is_identifying: bool,
    participants: Vec<(String, String)>, // (entity_name, cardinality)
    attr_positions: Vec<(Pt, String, ChenAttrKind)>,
}

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn render_chen_svg(doc: &ChenDocument) -> String {
    let cols = GRID_COLS.max(1);

    // ── Step 1: assign entity centers ────────────────────────────────────────
    let entity_layouts: Vec<EntityLayout> = doc
        .entities
        .iter()
        .enumerate()
        .map(|(idx, e)| {
            let col = idx % cols;
            let row = idx / cols;
            let cx = MARGIN + col as f64 * COL_SPACING;
            let cy = MARGIN + row as f64 * ROW_SPACING;
            let n_attrs = e.attrs.len();
            let attr_positions = compute_orbit_positions(cx, cy, ATTR_ORBIT_R, n_attrs)
                .into_iter()
                .zip(e.attrs.iter())
                .map(|(pt, attr)| (pt, attr.name.clone(), attr.kind))
                .collect();
            EntityLayout {
                name: e.name.clone(),
                cx,
                cy,
                is_weak: e.is_weak,
                attr_positions,
            }
        })
        .collect();

    // ── Step 2: assign relationship centers ──────────────────────────────────
    let rel_layouts: Vec<RelLayout> = doc
        .relationships
        .iter()
        .map(|rel| {
            let (cx, cy) = if rel.participants.is_empty() {
                let row = (entity_layouts.len() + cols - 1) / cols;
                (MARGIN, MARGIN + row as f64 * ROW_SPACING)
            } else {
                let mut sum_x = 0.0f64;
                let mut sum_y = 0.0f64;
                let mut matched = 0usize;
                for p in &rel.participants {
                    if let Some(el) = entity_layouts.iter().find(|e| e.name == p.entity) {
                        sum_x += el.cx;
                        sum_y += el.cy;
                        matched += 1;
                    }
                }
                if matched == 0 {
                    let row = (entity_layouts.len() + cols - 1) / cols;
                    (MARGIN, MARGIN + row as f64 * ROW_SPACING)
                } else {
                    // Offset slightly below the midpoint so it doesn't overlap entity boxes.
                    let mid_x = sum_x / matched as f64;
                    let mid_y = sum_y / matched as f64;
                    (mid_x, mid_y + 60.0)
                }
            };
            let n_attrs = rel.attrs.len();
            let attr_positions = compute_orbit_positions(cx, cy, ATTR_ORBIT_R, n_attrs)
                .into_iter()
                .zip(rel.attrs.iter())
                .map(|(pt, attr)| (pt, attr.name.clone(), attr.kind))
                .collect();
            RelLayout {
                name: rel.name.clone(),
                cx,
                cy,
                is_identifying: rel.is_identifying,
                participants: rel
                    .participants
                    .iter()
                    .map(|p| (p.entity.clone(), p.cardinality.clone()))
                    .collect(),
                attr_positions,
            }
        })
        .collect();

    // ── Step 3: compute canvas size ───────────────────────────────────────────
    let all_x: Vec<f64> = entity_layouts
        .iter()
        .map(|e| e.cx)
        .chain(rel_layouts.iter().map(|r| r.cx))
        .chain(
            entity_layouts
                .iter()
                .flat_map(|e| e.attr_positions.iter().map(|(pt, ..)| pt.x)),
        )
        .chain(
            rel_layouts
                .iter()
                .flat_map(|r| r.attr_positions.iter().map(|(pt, ..)| pt.x)),
        )
        .collect();
    let all_y: Vec<f64> = entity_layouts
        .iter()
        .map(|e| e.cy)
        .chain(rel_layouts.iter().map(|r| r.cy))
        .chain(
            entity_layouts
                .iter()
                .flat_map(|e| e.attr_positions.iter().map(|(pt, ..)| pt.y)),
        )
        .chain(
            rel_layouts
                .iter()
                .flat_map(|r| r.attr_positions.iter().map(|(pt, ..)| pt.y)),
        )
        .collect();

    let max_x = all_x.iter().cloned().fold(200.0f64, f64::max) + MARGIN + ATTR_RX + ENTITY_W;
    let max_y = all_y.iter().cloned().fold(100.0f64, f64::max) + MARGIN + ATTR_RY + ENTITY_H;
    let width = max_x.ceil() as i32;
    let height = max_y.ceil() as i32;

    // ── Step 4: emit SVG ─────────────────────────────────────────────────────
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title.
    if let Some(title) = &doc.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"28\" {} text-anchor=\"middle\">{}</text>",
            width / 2,
            FONT_TITLE,
            escape_text(title)
        ));
    }

    // ── Relationship-to-entity connecting lines + cardinality labels ─────────
    for rl in &rel_layouts {
        for (entity_name, cardinality) in &rl.participants {
            if let Some(el) = entity_layouts.iter().find(|e| e.name == *entity_name) {
                out.push_str(&segment_line(rl.cx, rl.cy, el.cx, el.cy, CONN_COLOR, 1.5));
                if !cardinality.is_empty() {
                    let label_x = lerp(rl.cx, el.cx, 0.8);
                    let label_y = lerp(rl.cy, el.cy, 0.8) - 6.0;
                    out.push_str(&format!(
                        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"#333333\" text-anchor=\"middle\">{}</text>",
                        label_x,
                        label_y,
                        FONT_ATTRS,
                        escape_text(cardinality)
                    ));
                }
            }
        }
    }

    // ── Entity attribute lines ───────────────────────────────────────────────
    for el in &entity_layouts {
        for (pt, ..) in &el.attr_positions {
            out.push_str(&segment_line(el.cx, el.cy, pt.x, pt.y, ATTR_LINE_COLOR, 1.0));
        }
    }

    // ── Relationship attribute lines ─────────────────────────────────────────
    for rl in &rel_layouts {
        for (pt, ..) in &rl.attr_positions {
            out.push_str(&segment_line(rl.cx, rl.cy, pt.x, pt.y, ATTR_LINE_COLOR, 1.0));
        }
    }

    // ── Entity rectangles ────────────────────────────────────────────────────
    for el in &entity_layouts {
        out.push_str(&render_entity(el));
    }

    // ── Relationship diamonds ────────────────────────────────────────────────
    for rl in &rel_layouts {
        out.push_str(&render_relationship(rl));
    }

    // ── Entity attribute ovals ───────────────────────────────────────────────
    for el in &entity_layouts {
        for (pt, name, kind) in &el.attr_positions {
            out.push_str(&render_attribute_oval(pt.x, pt.y, name, *kind));
        }
    }

    // ── Relationship attribute ovals ─────────────────────────────────────────
    for rl in &rel_layouts {
        for (pt, name, kind) in &rl.attr_positions {
            out.push_str(&render_attribute_oval(pt.x, pt.y, name, *kind));
        }
    }

    out.push_str("</svg>");
    out
}

// ─── Shape rendering helpers ──────────────────────────────────────────────────

fn render_entity(el: &EntityLayout) -> String {
    let x = el.cx - ENTITY_W / 2.0;
    let y = el.cy - ENTITY_H / 2.0;
    let mut s = String::new();
    // Outer double-border rect for weak entities.
    if el.is_weak {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"white\" stroke=\"{}\" stroke-width=\"3.5\"/>",
            x - 4.0,
            y - 4.0,
            ENTITY_W + 8.0,
            ENTITY_H + 8.0,
            WEAK_STROKE
        ));
    }
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        x, y, ENTITY_W, ENTITY_H, ENTITY_FILL, ENTITY_STROKE
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
        el.cx,
        el.cy,
        FONT_ATTRS,
        ENTITY_TEXT,
        escape_text(&el.name)
    ));
    s
}

fn render_relationship(rl: &RelLayout) -> String {
    let hw = DIAMOND_HALF_W;
    let hh = DIAMOND_HALF_H;
    let cx = rl.cx;
    let cy = rl.cy;
    // Diamond points: top, right, bottom, left.
    let points = format!(
        "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
        cx,
        cy - hh,
        cx + hw,
        cy,
        cx,
        cy + hh,
        cx - hw,
        cy
    );
    let mut s = String::new();
    // Double-border for identifying relationships.
    if rl.is_identifying {
        let pts2 = format!(
            "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
            cx,
            cy - hh - 5.0,
            cx + hw + 5.0,
            cy,
            cx,
            cy + hh + 5.0,
            cx - hw - 5.0,
            cy
        );
        s.push_str(&format!(
            "<polygon points=\"{}\" fill=\"white\" stroke=\"{}\" stroke-width=\"2\"/>",
            pts2, REL_STROKE
        ));
    }
    s.push_str(&format!(
        "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        points, REL_FILL, REL_STROKE
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
        cx,
        cy,
        FONT_ATTRS,
        REL_TEXT,
        escape_text(&rl.name)
    ));
    s
}

fn render_attribute_oval(cx: f64, cy: f64, name: &str, kind: ChenAttrKind) -> String {
    let stroke_dash = if kind == ChenAttrKind::Derived {
        " stroke-dasharray=\"4 3\""
    } else {
        ""
    };

    let mut s = String::new();
    // Double oval for multivalued.
    if kind == ChenAttrKind::Multivalued {
        s.push_str(&format!(
            "<ellipse cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            cx,
            cy,
            ATTR_RX + 4.0,
            ATTR_RY + 4.0,
            ATTR_STROKE
        ));
    }
    s.push_str(&format!(
        "<ellipse cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>" ,
        cx, cy, ATTR_RX, ATTR_RY, ATTR_FILL, ATTR_STROKE, stroke_dash
    ));

    // Key attributes get underlined text.
    let decoration = if kind == ChenAttrKind::Key {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\"{}>{}</text>",
        cx, cy, FONT_ATTRS, ATTR_TEXT, decoration, escape_text(name)
    ));
    s
}

fn segment_line(x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64) -> String {
    format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"{:.1}\"/>",
        x1, y1, x2, y2, color, width
    )
}

// ─── Layout utilities ─────────────────────────────────────────────────────────

/// Compute `n` evenly-spaced points on a circle of radius `r` around `(cx, cy)`.
/// When `n == 0` returns empty. Starts from the top and goes clockwise.
fn compute_orbit_positions(cx: f64, cy: f64, r: f64, n: usize) -> Vec<Pt> {
    if n == 0 {
        return Vec::new();
    }
    let step = std::f64::consts::TAU / n as f64;
    (0..n)
        .map(|i| {
            let angle = -std::f64::consts::FRAC_PI_2 + i as f64 * step;
            Pt::new(cx + r * angle.cos(), cy + r * angle.sin())
        })
        .collect()
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

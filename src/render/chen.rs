/// Chen / EER entity-relationship diagram renderer.
///
/// Visual conventions:
///   - Entity         - rectangle (double-border for weak entities)
///   - Attribute      - oval; key attributes have underlined text
///   - Relationship   - diamond (double-border for identifying relationships)
///   - Lines          - straight line connecting shapes
///
/// Layout strategy (Wave-55 polish):
///   - Entities placed in a grid (3 columns max), 200 px cell size.
///   - Relationships placed between each pair of participating entities
///     (centroid of participant bounding box + perpendicular offset).
///   - Attributes placed around the perimeter of their owner (entity or
///     relationship) in 8 directional slots at sufficient clearance from
///     entity bbox so ovals never overlap the entity rectangle.
///   - Each attribute oval connects to its owner via a thin line from
///     the oval edge (perimeter-clipped) to the entity bbox edge.
///   - Relationship diamonds connect to each participating entity via a
///     line; cardinality labels appear 65% along the line from diamond
///     toward entity (near the entity endpoint).
use super::svg::escape_text;
use crate::model::{ChenAttrKind, ChenDocument, ChenRelationship};

// ─── Layout constants ─────────────────────────────────────────────────────────

const ENTITY_W: f64 = 120.0;
const ENTITY_H: f64 = 44.0;
const ATTR_RX: f64 = 40.0; // horizontal radius of attribute oval
const ATTR_RY: f64 = 18.0; // vertical radius
const DIAMOND_HALF_W: f64 = 54.0;
const DIAMOND_HALF_H: f64 = 24.0;
const GRID_COLS: usize = 3;
const COL_SPACING: f64 = 300.0; // horizontal distance between entity centers
const ROW_SPACING: f64 = 230.0; // vertical distance between entity rows
/// Left/top margin = distance from canvas edge to first entity center.
/// orbit_r ≈ 141 for entity attrs.  Using 200 ensures attrs in any direction
/// land at x ≥ 200 - 141 = 59, which with rx=40 gives left edge at 19px.
/// The canvas-bound rotation loop then pushes the ring to keep edges ≥ 30px in.
const MARGIN: f64 = 200.0;

/// Clearance from entity bbox edge to attribute oval center (minimum gap so
/// the oval border does not touch the entity rectangle border).
/// Must be large enough to compensate for diagonal-exit corner effects.
const ATTR_CLEARANCE: f64 = 25.0;

const FONT_ATTRS: &str = "font-family=\"monospace\" font-size=\"13\"";
const FONT_TITLE: &str = "font-family=\"monospace\" font-size=\"16\" font-weight=\"bold\"";

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

/// Intermediate relationship data: (name, cx, cy, is_identifying, participants).
/// The participants list is (entity_name, cardinality) pairs.
type RelCenter = (String, f64, f64, bool, Vec<(String, String)>);

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn render_chen_svg(doc: &ChenDocument) -> String {
    let cols = GRID_COLS.max(1);

    // ── Step 1: assign entity centers ────────────────────────────────────────
    // First pass: compute entity centers without attributes (needed for
    // relationship placement which then feeds attribute slot selection).
    let entity_centers: Vec<(String, f64, f64, bool)> = doc
        .entities
        .iter()
        .enumerate()
        .map(|(idx, e)| {
            let col = idx % cols;
            let row = idx / cols;
            let cx = MARGIN + col as f64 * COL_SPACING;
            let cy = MARGIN + row as f64 * ROW_SPACING;
            (e.name.clone(), cx, cy, e.is_weak)
        })
        .collect();

    // ── Step 2: assign relationship centers ──────────────────────────────────
    // Compute rel centers early so we know the "diamond direction" per entity.
    let rel_centers: Vec<RelCenter> = doc
        .relationships
        .iter()
        .map(|rel| {
            let (cx, cy) = compute_rel_center(rel, &entity_centers, cols);
            let participants = rel
                .participants
                .iter()
                .map(|p| (p.entity.clone(), p.cardinality.clone()))
                .collect();
            (rel.name.clone(), cx, cy, rel.is_identifying, participants)
        })
        .collect();

    // ── Step 3: assign attribute positions with clearance ────────────────────
    let entity_layouts: Vec<EntityLayout> = doc
        .entities
        .iter()
        .zip(entity_centers.iter())
        .map(|(e, (_, cx, cy, _))| {
            let cx = *cx;
            let cy = *cy;
            // Collect directions toward each relationship this entity participates in.
            let rel_dirs: Vec<(f64, f64)> = rel_centers
                .iter()
                .filter(|(_, _, _, _, participants)| {
                    participants.iter().any(|(en, _)| en == &e.name)
                })
                .map(|(_, rcx, rcy, _, _)| {
                    let dx = rcx - cx;
                    let dy = rcy - cy;
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    (dx / len, dy / len)
                })
                .collect();

            let n_attrs = e.attrs.len();
            let attr_positions =
                compute_entity_attr_positions(cx, cy, ENTITY_W, ENTITY_H, n_attrs, &rel_dirs, 0.0)
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

    // ── Step 4: build full RelLayout with relationship attrs ─────────────────
    let rel_layouts: Vec<RelLayout> = doc
        .relationships
        .iter()
        .zip(rel_centers.iter())
        .map(|(rel, (_, cx, cy, _, participants))| {
            let cx = *cx;
            let cy = *cy;
            let n_attrs = rel.attrs.len();
            // Relationship attributes orbit the diamond center.
            // Avoid directions toward participant entities.
            let rel_dirs: Vec<(f64, f64)> = participants
                .iter()
                .filter_map(|(en, _)| {
                    entity_layouts.iter().find(|e| &e.name == en).map(|el| {
                        let dx = el.cx - cx;
                        let dy = el.cy - cy;
                        let len = (dx * dx + dy * dy).sqrt().max(1.0);
                        (dx / len, dy / len)
                    })
                })
                .collect();
            let attr_positions = compute_entity_attr_positions(
                cx,
                cy,
                DIAMOND_HALF_W * 2.0,
                DIAMOND_HALF_H * 2.0,
                n_attrs,
                &rel_dirs,
                0.0,
            )
            .into_iter()
            .zip(rel.attrs.iter())
            .map(|(pt, attr)| (pt, attr.name.clone(), attr.kind))
            .collect();
            RelLayout {
                name: rel.name.clone(),
                cx,
                cy,
                is_identifying: rel.is_identifying,
                participants: participants.clone(),
                attr_positions,
            }
        })
        .collect();

    // ── Step 5: compute canvas size ───────────────────────────────────────────
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
    let max_y = all_y.iter().cloned().fold(200.0f64, f64::max) + MARGIN + ATTR_RY + ENTITY_H;

    // Extra space for title.
    let title_h = if doc.title.is_some() { 40.0 } else { 0.0 };
    let width = max_x;
    let height = max_y + title_h;

    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">\n",
        width, height, width, height
    );
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n");

    // Title.
    if let Some(title) = &doc.title {
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"28\" {} fill=\"#111827\" text-anchor=\"middle\">{}</text>\n",
            width / 2.0,
            FONT_TITLE,
            escape_text(title)
        ));
    }

    // Shift all diagram elements down by title_h.
    let dy = title_h;

    // ── Relationship-to-entity connection lines ────────────────────────────
    // Drawn first (below everything else) so entity rects occlude the line
    // end that enters the entity bbox.
    for rl in &rel_layouts {
        for (entity_name, cardinality) in &rl.participants {
            if let Some(el) = entity_layouts.iter().find(|e| e.name == *entity_name) {
                // Line from entity center to relationship diamond center.
                // Clip endpoints to entity bbox edge and diamond perimeter so
                // the line doesn't visually go through shapes.
                let (ex, ey) =
                    clip_to_entity_edge(el.cx, el.cy, rl.cx, rl.cy + dy, el.cx, el.cy + dy);
                let (rx, ry) = clip_to_diamond_edge(rl.cx, rl.cy + dy, el.cx, el.cy + dy);
                out.push_str(&segment_line(ex, ey, rx, ry, CONN_COLOR, 1.5));

                // Cardinality label: 65% along from diamond toward entity (near entity).
                if !cardinality.is_empty() {
                    let t = 0.65_f64;
                    let label_x = rl.cx + (el.cx - rl.cx) * t;
                    let label_y = (rl.cy + dy) + (el.cy + dy - (rl.cy + dy)) * t - 8.0;
                    out.push_str(&format!(
                        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\">{}</text>\n",
                        label_x,
                        label_y,
                        FONT_ATTRS,
                        CONN_COLOR,
                        escape_text(cardinality)
                    ));
                }
            }
        }
    }

    // ── Entity attribute lines ────────────────────────────────────────────────
    for el in &entity_layouts {
        for (pt, ..) in &el.attr_positions {
            // Clip the entity end to the entity bbox edge.
            let (ex, ey) =
                clip_to_entity_edge(el.cx, el.cy + dy, pt.x, pt.y + dy, el.cx, el.cy + dy);
            // Clip the oval end to the oval perimeter.
            let (ax, ay) = clip_to_oval_edge(pt.x, pt.y + dy, el.cx, el.cy + dy);
            out.push_str(&segment_line(ex, ey, ax, ay, ATTR_LINE_COLOR, 1.0));
        }
    }

    // ── Relationship attribute lines ─────────────────────────────────────────
    for rl in &rel_layouts {
        for (pt, ..) in &rl.attr_positions {
            let (rx, ry) = clip_to_diamond_edge(rl.cx, rl.cy + dy, pt.x, pt.y + dy);
            let (ax, ay) = clip_to_oval_edge(pt.x, pt.y + dy, rl.cx, rl.cy + dy);
            out.push_str(&segment_line(rx, ry, ax, ay, ATTR_LINE_COLOR, 1.0));
        }
    }

    // ── Entity rectangles ────────────────────────────────────────────────────
    for el in &entity_layouts {
        out.push_str(&render_entity(el, dy));
    }

    // ── Relationship diamonds ────────────────────────────────────────────────
    for rl in &rel_layouts {
        out.push_str(&render_relationship(rl, dy));
    }

    // ── Entity attribute ovals ───────────────────────────────────────────────
    for el in &entity_layouts {
        for (pt, name, kind) in &el.attr_positions {
            out.push_str(&render_attribute_oval(pt.x, pt.y + dy, name, *kind));
        }
    }

    // ── Relationship attribute ovals ─────────────────────────────────────────
    for rl in &rel_layouts {
        for (pt, name, kind) in &rl.attr_positions {
            out.push_str(&render_attribute_oval(pt.x, pt.y + dy, name, *kind));
        }
    }

    out.push_str("</svg>");
    out
}

// ─── Shape rendering helpers ──────────────────────────────────────────────────

fn render_entity(el: &EntityLayout, dy: f64) -> String {
    let x = el.cx - ENTITY_W / 2.0;
    let y = el.cy - ENTITY_H / 2.0 + dy;
    let mut s = String::new();
    // Outer double-border rect for weak entities.
    if el.is_weak {
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"white\" stroke=\"{}\" stroke-width=\"3.5\"/>\n",
            x - 4.0,
            y - 4.0,
            ENTITY_W + 8.0,
            ENTITY_H + 8.0,
            WEAK_STROKE
        ));
    }
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>\n",
        x, y, ENTITY_W, ENTITY_H, ENTITY_FILL, ENTITY_STROKE
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>\n",
        el.cx,
        el.cy + dy,
        FONT_ATTRS,
        ENTITY_TEXT,
        escape_text(&el.name)
    ));
    s
}

fn render_relationship(rl: &RelLayout, dy: f64) -> String {
    let hw = DIAMOND_HALF_W;
    let hh = DIAMOND_HALF_H;
    let cx = rl.cx;
    let cy = rl.cy + dy;
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
            "<polygon points=\"{}\" fill=\"white\" stroke=\"{}\" stroke-width=\"2\"/>\n",
            pts2, REL_STROKE
        ));
    }
    s.push_str(&format!(
        "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>\n",
        points, REL_FILL, REL_STROKE
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>\n",
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
            "<ellipse cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>\n",
            cx,
            cy,
            ATTR_RX + 4.0,
            ATTR_RY + 4.0,
            ATTR_STROKE
        ));
    }
    s.push_str(&format!(
        "<ellipse cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>\n",
        cx, cy, ATTR_RX, ATTR_RY, ATTR_FILL, ATTR_STROKE, stroke_dash
    ));

    // Key attributes get underlined text.
    let decoration = if kind == ChenAttrKind::Key {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\"{}>{}</text>\n",
        cx, cy, FONT_ATTRS, ATTR_TEXT, decoration, escape_text(name)
    ));
    s
}

fn segment_line(x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64) -> String {
    format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"{:.1}\"/>\n",
        x1, y1, x2, y2, color, width
    )
}

// ─── Layout utilities ─────────────────────────────────────────────────────────

/// Compute relationship center: centroid of participating entities + perpendicular
/// offset to avoid sitting directly on the line between them.
fn compute_rel_center(
    rel: &ChenRelationship,
    entity_centers: &[(String, f64, f64, bool)],
    cols: usize,
) -> (f64, f64) {
    if rel.participants.is_empty() {
        let row = entity_centers.len().div_ceil(cols);
        return (MARGIN, MARGIN + row as f64 * ROW_SPACING);
    }
    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    let mut matched = 0usize;
    for p in &rel.participants {
        if let Some((_, ex, ey, _)) = entity_centers.iter().find(|(n, ..)| n == &p.entity) {
            sum_x += ex;
            sum_y += ey;
            matched += 1;
        }
    }
    if matched == 0 {
        let row = entity_centers.len().div_ceil(cols);
        return (MARGIN, MARGIN + row as f64 * ROW_SPACING);
    }
    let mid_x = sum_x / matched as f64;
    let mid_y = sum_y / matched as f64;
    // If relationship connects exactly 2 entities, offset perpendicular to their
    // connecting line so the diamond doesn't sit on the entity-to-entity line.
    // For 1 or 3+ entities, offset downward by a fixed amount.
    if matched == 2 {
        let fallback1 = (String::new(), mid_x, mid_y, false);
        let fallback2 = (String::new(), mid_x, mid_y, false);
        let (_, x1, y1, _) = entity_centers
            .iter()
            .find(|(n, ..)| n == &rel.participants[0].entity)
            .unwrap_or(&fallback1);
        let (_, x2, y2, _) = entity_centers
            .iter()
            .find(|(n, ..)| n == &rel.participants[1].entity)
            .unwrap_or(&fallback2);
        let dx = x2 - x1;
        let dy_vec = y2 - y1;
        let len = (dx * dx + dy_vec * dy_vec).sqrt().max(1.0);
        // Perpendicular unit vector (rotate 90° clockwise): (dy_vec/len, -dx/len)
        // Offset by ROW_SPACING * 0.35 toward lower-right to push diamond below center.
        let offset = ROW_SPACING * 0.35;
        // Choose the perpendicular direction that has a positive y component
        // (preferring "below" placement so diamond appears between entities and below).
        let (perp_x, perp_y) = if -dx / len >= 0.0 {
            (dy_vec / len, -dx / len)
        } else {
            (-dy_vec / len, dx / len)
        };
        // If entities are in the same row (same y), push diamond below.
        // If entities are in different rows, push to the right of the midpoint.
        let (off_x, off_y) = if (dy_vec / len).abs() < 0.3 {
            // Mostly horizontal — push below
            (0.0, offset)
        } else {
            // Diagonal or vertical — use perpendicular + below bias
            (perp_x * offset * 0.5, perp_y.abs() * offset + 20.0)
        };
        (mid_x + off_x, mid_y + off_y)
    } else {
        (mid_x, mid_y + 60.0)
    }
}

/// Compute attribute oval centers for an owner at (cx, cy) with half-extents
/// (owner_w × owner_h).
///
/// Strategy:
///   - N attributes are placed evenly around the owner at angles
///     `start_angle + i*(TAU/N)`, where `start_angle` is chosen to put the
///     first attribute in the direction MOST OPPOSITE to the average relationship
///     direction.
///   - Oval center distance from owner center is computed so the oval border
///     clears the owner bbox by ATTR_CLEARANCE pixels.
///   - If any oval center would be off-canvas (x < min_canvas_x + 2*ATTR_RX),
///     the ring is rotated by TAU/16 steps until all ovals are on-canvas, up to
///     a full rotation (at which point the best-effort position is used).
fn compute_entity_attr_positions(
    cx: f64,
    cy: f64,
    owner_w: f64,
    owner_h: f64,
    n: usize,
    rel_dirs: &[(f64, f64)],
    min_canvas_x: f64,
) -> Vec<Pt> {
    if n == 0 {
        return Vec::new();
    }

    let half_w = owner_w / 2.0;
    let half_h = owner_h / 2.0;

    // Oval center must stay far enough from canvas edges that the full oval
    // and its text label are comfortably visible.  Use 30px from left/top.
    let min_oval_cx = min_canvas_x + ATTR_RX + 30.0;
    let min_oval_cy = ATTR_RY + 30.0;

    // Orbit radius: the distance from the owner center to the attribute oval center.
    // Computed as the diagonal of the (half_w + clearance + oval_rx) × (half_h + clearance + oval_ry)
    // bounding box, which guarantees the oval bbox clears the owner bbox in ALL directions
    // including diagonals (the worst case for axis-aligned clearance).
    let orbit_r = ((half_w + ATTR_CLEARANCE + ATTR_RX).powi(2)
        + (half_h + ATTR_CLEARANCE + ATTR_RY).powi(2))
    .sqrt();

    // Helper: given a direction angle, compute the oval center position at orbit_r.
    let oval_center_for_angle = |angle: f64| -> Pt {
        let ux = angle.cos();
        let uy = angle.sin();
        Pt::new(cx + ux * orbit_r, cy + uy * orbit_r)
    };

    // Find start angle: direction most opposed to the average relationship direction.
    let start_angle = if rel_dirs.is_empty() {
        // No relationships: start at top (-PI/2).
        -std::f64::consts::FRAC_PI_2
    } else {
        // Average relationship direction.
        let avg_rdx: f64 = rel_dirs.iter().map(|&(rx, _)| rx).sum::<f64>() / rel_dirs.len() as f64;
        let avg_rdy: f64 = rel_dirs.iter().map(|&(_, ry)| ry).sum::<f64>() / rel_dirs.len() as f64;
        // Opposite direction of average: negate both components.
        // In Rust, atan2(y, x) → the angle is the second argument (x) axis reference.
        // f64::atan2(self, other) = atan2(self=y, other=x).
        (-avg_rdy).atan2(-avg_rdx)
    };

    // Angular spacing between attributes.
    let attr_step = if n > 1 {
        std::f64::consts::TAU / n as f64
    } else {
        0.0
    };

    // Try ring rotations in small steps (TAU/32 ≈ 11.25°) until all ovals are
    // on-canvas.  Give up after a full rotation and use whatever we have.
    let n_rot_steps = 32usize;
    let rot_step = std::f64::consts::TAU / n_rot_steps as f64;

    for rot in 0..=n_rot_steps {
        let offset = rot as f64 * rot_step;
        let positions: Vec<Pt> = (0..n)
            .map(|i| {
                let angle = start_angle + offset + i as f64 * attr_step;
                oval_center_for_angle(angle)
            })
            .collect();

        // Check all ovals are on-canvas.
        let all_ok = positions
            .iter()
            .all(|p| p.x >= min_oval_cx && p.y >= min_oval_cy);

        if all_ok || rot == n_rot_steps {
            return positions;
        }
    }

    // Should never reach here, but return top placement as fallback.
    (0..n)
        .map(|i| {
            let angle = start_angle + i as f64 * attr_step;
            oval_center_for_angle(angle)
        })
        .collect()
}

// ─── Perimeter-clip helpers ───────────────────────────────────────────────────

/// Return the point on the entity bbox boundary (centered at ecx,ecy with
/// half-extents ENTITY_W/2 × ENTITY_H/2) that lies on the line from (ecx,ecy)
/// toward (tx,ty).  Falls back to center if the direction is zero.
fn clip_to_entity_edge(ecx: f64, ecy: f64, tx: f64, ty: f64, _cx: f64, _cy: f64) -> (f64, f64) {
    let dx = tx - ecx;
    let dy = ty - ecy;
    let half_w = ENTITY_W / 2.0;
    let half_h = ENTITY_H / 2.0;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (ecx, ecy);
    }
    // Parametric ray: (ecx + t*dx, ecy + t*dy).  Find smallest t>0 hitting bbox.
    let tx_hit = if dx.abs() > 1e-9 {
        (half_w * dx.signum() / dx).abs()
    } else {
        f64::INFINITY
    };
    let ty_hit = if dy.abs() > 1e-9 {
        (half_h * dy.signum() / dy).abs()
    } else {
        f64::INFINITY
    };
    let t = tx_hit.min(ty_hit);
    (ecx + t * dx, ecy + t * dy)
}

/// Return the point on the diamond perimeter (centered at dcx,dcy with
/// half-extents DIAMOND_HALF_W × DIAMOND_HALF_H) on the line toward (tx,ty).
fn clip_to_diamond_edge(dcx: f64, dcy: f64, tx: f64, ty: f64) -> (f64, f64) {
    let dx = tx - dcx;
    let dy = ty - dcy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (dcx, dcy);
    }
    let hw = DIAMOND_HALF_W;
    let hh = DIAMOND_HALF_H;
    // Diamond inequality: |x/hw| + |y/hh| ≤ 1 → t = 1 / (|dx|/hw + |dy|/hh).
    let denom = dx.abs() / hw + dy.abs() / hh;
    if denom < 1e-9 {
        return (dcx, dcy);
    }
    let t = 1.0 / denom;
    (dcx + t * dx, dcy + t * dy)
}

/// Return the point on the oval perimeter (centered at ocx,ocy with radii
/// ATTR_RX × ATTR_RY) on the line toward (tx,ty).
fn clip_to_oval_edge(ocx: f64, ocy: f64, tx: f64, ty: f64) -> (f64, f64) {
    let dx = tx - ocx;
    let dy = ty - ocy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (ocx, ocy);
    }
    // Ellipse: (x/rx)² + (y/ry)² = 1.  On the ray x=t*dx, y=t*dy:
    //   t = 1 / sqrt((dx/rx)² + (dy/ry)²)
    let rx = ATTR_RX;
    let ry = ATTR_RY;
    let t = 1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt().max(1e-9);
    (ocx + t * dx, ocy + t * dy)
}

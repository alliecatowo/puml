/// Chen / EER entity-relationship diagram renderer.
///
/// Visual conventions:
///   - Entity       - rectangle (double-border for weak entities)
///   - Attribute    - oval; key attributes have underlined text
///   - Relationship - diamond (double-border for identifying relationships)
///
/// Layout strategy:
///   1. Place entities and relationships as a bipartite graph.
///   2. Place entity attributes on the least congested side of each entity.
///   3. Place relationship attributes away from participating entities.
///   4. Place cardinality labels near entity endpoints with normal offsets.
use std::collections::BTreeMap;

use super::graph_layout::{
    layout_hierarchical, Direction, EdgeSpec as GlEdgeSpec, LayoutOptions as GlOptions,
    NodeSize as GlNodeSize,
};
use super::scene_graph::{estimate_text_bbox, Rect as SceneRect};
use super::svg::escape_text;
use crate::model::{ChenAttrKind, ChenDocument};

// ─── Layout constants ─────────────────────────────────────────────────────────

const ENTITY_MIN_W: f64 = 120.0;
const ENTITY_H: f64 = 44.0;
const ENTITY_PAD_X: f64 = 26.0;
const ATTR_MIN_W: f64 = 80.0;
const ATTR_RY: f64 = 18.0;
const ATTR_PAD_X: f64 = 24.0;
const ATTR_GAP_X: f64 = 18.0;
const ATTR_GAP_Y: f64 = 16.0;
const ATTR_SIDE_GAP: f64 = 30.0;
const DIAMOND_MIN_HALF_W: f64 = 54.0;
const DIAMOND_HALF_H: f64 = 25.0;
const DIAMOND_PAD_X: f64 = 26.0;
const CONTENT_MARGIN: f64 = 32.0;
const TITLE_H: f64 = 40.0;
const COLLISION_CLEARANCE: f64 = 8.0;
const LABEL_CLEARANCE: f64 = 6.0;

const FONT_ATTRS: &str = "font-family=\"monospace\" font-size=\"13\"";
const FONT_TITLE: &str = "font-family=\"monospace\" font-size=\"16\" font-weight=\"bold\"";

// Entity fill/stroke
const ENTITY_FILL: &str = "#dbeafe";
const ENTITY_STROKE: &str = "#2563eb";
const ENTITY_TEXT: &str = "#1e3a8a";
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

#[derive(Debug, Clone, Copy)]
struct Pt {
    x: f64,
    y: f64,
}

impl Pt {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn from_center(cx: f64, cy: f64, w: f64, h: f64) -> Self {
        Self {
            x: cx - w / 2.0,
            y: cy - h / 2.0,
            w,
            h,
        }
    }

    fn right(self) -> f64 {
        self.x + self.w
    }

    fn bottom(self) -> f64 {
        self.y + self.h
    }

    fn center(self) -> Pt {
        Pt::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Debug, Clone)]
struct EntityLayout {
    name: String,
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    is_weak: bool,
    attr_positions: Vec<AttrLayout>,
}

impl EntityLayout {
    fn bounds(&self) -> Rect {
        let base = Rect::from_center(self.cx, self.cy, self.w, self.h);
        if self.is_weak {
            Rect {
                x: base.x - 4.0,
                y: base.y - 4.0,
                w: base.w + 8.0,
                h: base.h + 8.0,
            }
        } else {
            base
        }
    }
}

#[derive(Debug, Clone)]
struct RelLayout {
    name: String,
    cx: f64,
    cy: f64,
    half_w: f64,
    half_h: f64,
    is_identifying: bool,
    participants: Vec<(String, String)>,
    attr_positions: Vec<AttrLayout>,
}

impl RelLayout {
    fn bounds(&self) -> Rect {
        let pad = if self.is_identifying { 5.0 } else { 0.0 };
        Rect::from_center(
            self.cx,
            self.cy,
            self.half_w * 2.0 + pad * 2.0,
            self.half_h * 2.0 + pad * 2.0,
        )
    }
}

#[derive(Debug, Clone)]
struct AttrLayout {
    owner_kind: &'static str,
    owner_name: String,
    name: String,
    kind: ChenAttrKind,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
}

impl AttrLayout {
    fn bounds(&self) -> Rect {
        let outline = if self.kind == ChenAttrKind::Multivalued {
            4.0
        } else {
            0.0
        };
        Rect::from_center(
            self.cx,
            self.cy,
            (self.rx + outline) * 2.0,
            (self.ry + outline) * 2.0,
        )
    }

    fn id(&self) -> String {
        format!("{}:{}:{}", self.owner_kind, self.owner_name, self.name)
    }
}

#[derive(Debug, Clone)]
struct CardinalityLayout {
    rel_name: String,
    entity_name: String,
    label: String,
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
}

impl CardinalityLayout {
    fn bounds(&self) -> Rect {
        Rect::from_center(self.cx, self.cy, self.w, self.h)
    }
}

#[derive(Debug, Clone, Copy)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn render_chen_svg(doc: &ChenDocument) -> String {
    let mut layout = build_primary_layout(doc);
    place_all_attributes(doc, &mut layout);
    let cardinalities = place_cardinality_labels(&layout);
    let (shift_x, shift_y, width, height) =
        canvas_transform(&layout, &cardinalities, doc.title.is_some());

    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">\n",
        width, height, width, height
    );
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n");

    if let Some(title) = &doc.title {
        let title_bbox = estimate_text_bbox(width / 2.0, 28.0, title, 16.0, true).as_puml_bbox();
        out.push_str(&format!(
            "<text class=\"chen-title puml-label\" data-puml-owner=\"diagram\" data-puml-label-kind=\"title\" data-puml-bbox=\"{}\" x=\"{:.1}\" y=\"28\" {} fill=\"#111827\" text-anchor=\"middle\">{}</text>\n",
            title_bbox,
            width / 2.0,
            FONT_TITLE,
            escape_text(title)
        ));
    }

    render_relationship_lines(&mut out, &layout, shift_x, shift_y);
    render_attribute_lines(&mut out, &layout, shift_x, shift_y);

    for entity in &layout.entities {
        out.push_str(&render_entity(entity, shift_x, shift_y));
    }
    for rel in &layout.relationships {
        out.push_str(&render_relationship(rel, shift_x, shift_y));
    }
    for entity in &layout.entities {
        for attr in &entity.attr_positions {
            out.push_str(&render_attribute_oval(attr, shift_x, shift_y));
        }
    }
    for rel in &layout.relationships {
        for attr in &rel.attr_positions {
            out.push_str(&render_attribute_oval(attr, shift_x, shift_y));
        }
    }
    for label in &cardinalities {
        out.push_str(&render_cardinality_label(label, shift_x, shift_y));
    }

    out.push_str("</svg>");
    out
}

// ─── Primary graph layout ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ChenLayout {
    entities: Vec<EntityLayout>,
    relationships: Vec<RelLayout>,
}

fn build_primary_layout(doc: &ChenDocument) -> ChenLayout {
    let mut gl_nodes: Vec<GlNodeSize> = Vec::new();

    let entity_layout_ids: BTreeMap<String, String> = doc
        .entities
        .iter()
        .enumerate()
        .map(|(idx, entity)| (entity.name.clone(), entity_id(idx, &entity.name)))
        .collect();

    for (idx, entity) in doc.entities.iter().enumerate() {
        let (w, h) = entity_size(&entity.name);
        gl_nodes.push(GlNodeSize {
            id: entity_id(idx, &entity.name),
            width: w,
            height: h,
            parent: None,
        });
    }

    for (idx, rel) in doc.relationships.iter().enumerate() {
        let (half_w, half_h) = relationship_size(&rel.name);
        gl_nodes.push(GlNodeSize {
            id: rel_id(idx, &rel.name),
            width: half_w * 2.0,
            height: half_h * 2.0,
            parent: None,
        });
    }

    let mut gl_edges: Vec<GlEdgeSpec> = Vec::new();
    for (rel_idx, rel) in doc.relationships.iter().enumerate() {
        for (part_idx, participant) in rel.participants.iter().enumerate() {
            if let Some(entity_id) = entity_layout_ids.get(&participant.entity) {
                gl_edges.push(GlEdgeSpec {
                    id: format!("chen-edge-{rel_idx}-{part_idx}"),
                    from: entity_id.clone(),
                    to: rel_id(rel_idx, &rel.name),
                });
            }
        }
    }

    let gl = layout_hierarchical(
        &gl_nodes,
        &gl_edges,
        &GlOptions {
            rank_separation: 140.0,
            node_separation: 110.0,
            group_padding: 0.0,
            direction: Direction::TopDown,
            canvas_margin: 80.0,
            canvas_right_margin: None,
        },
    );

    let entities = doc
        .entities
        .iter()
        .enumerate()
        .map(|(idx, entity)| {
            let (w, h) = entity_size(&entity.name);
            let (x, y) = gl
                .node_positions
                .get(&entity_id(idx, &entity.name))
                .copied()
                .unwrap_or((80.0, 80.0));
            EntityLayout {
                name: entity.name.clone(),
                cx: x + w / 2.0,
                cy: y + h / 2.0,
                w,
                h,
                is_weak: entity.is_weak,
                attr_positions: Vec::new(),
            }
        })
        .collect();

    let relationships = doc
        .relationships
        .iter()
        .enumerate()
        .map(|(idx, rel)| {
            let (half_w, half_h) = relationship_size(&rel.name);
            let (x, y) = gl
                .node_positions
                .get(&rel_id(idx, &rel.name))
                .copied()
                .unwrap_or((80.0 + idx as f64 * 180.0, 270.0));
            RelLayout {
                name: rel.name.clone(),
                cx: x + half_w,
                cy: y + half_h,
                half_w,
                half_h,
                is_identifying: rel.is_identifying,
                participants: rel
                    .participants
                    .iter()
                    .map(|p| (p.entity.clone(), p.cardinality.clone()))
                    .collect(),
                attr_positions: Vec::new(),
            }
        })
        .collect();

    ChenLayout {
        entities,
        relationships,
    }
}

// ─── Attribute and cardinality placement ──────────────────────────────────────

fn place_all_attributes(doc: &ChenDocument, layout: &mut ChenLayout) {
    let mut occupied = primary_bounds(layout);

    for (idx, entity) in doc.entities.iter().enumerate() {
        let attrs = place_attrs_for_owner(
            "entity",
            &entity.name,
            &entity.attrs,
            layout.entities[idx].bounds(),
            &[Side::Top, Side::Left, Side::Right, Side::Bottom],
            &occupied,
        );
        occupied.extend(attrs.iter().map(AttrLayout::bounds));
        layout.entities[idx].attr_positions = attrs;
    }

    for (idx, rel) in doc.relationships.iter().enumerate() {
        let attrs = place_attrs_for_owner(
            "relationship",
            &rel.name,
            &rel.attrs,
            layout.relationships[idx].bounds(),
            &[Side::Bottom, Side::Top, Side::Left, Side::Right],
            &occupied,
        );
        occupied.extend(attrs.iter().map(AttrLayout::bounds));
        layout.relationships[idx].attr_positions = attrs;
    }
}

fn place_attrs_for_owner(
    owner_kind: &'static str,
    owner_name: &str,
    attrs: &[crate::model::ChenAttr],
    owner: Rect,
    sides: &[Side],
    occupied: &[Rect],
) -> Vec<AttrLayout> {
    if attrs.is_empty() {
        return Vec::new();
    }

    sides
        .iter()
        .enumerate()
        .map(|(preference_idx, side)| {
            let positions = attr_positions_for_side(owner_kind, owner_name, attrs, owner, *side);
            let score = placement_score(&positions, occupied, preference_idx);
            (score, positions)
        })
        .min_by(|(a_score, _), (b_score, _)| {
            a_score
                .partial_cmp(b_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, positions)| positions)
        .unwrap_or_default()
}

fn attr_positions_for_side(
    owner_kind: &'static str,
    owner_name: &str,
    attrs: &[crate::model::ChenAttr],
    owner: Rect,
    side: Side,
) -> Vec<AttrLayout> {
    let sized_attrs: Vec<(&crate::model::ChenAttr, f64, f64)> = attrs
        .iter()
        .map(|attr| {
            let (rx, ry) = attribute_radii(&attr.name);
            (attr, rx, ry)
        })
        .collect();

    match side {
        Side::Top | Side::Bottom => {
            let max_cols = sized_attrs.len().clamp(1, 3);
            let mut result = Vec::new();
            for (row_idx, row) in sized_attrs.chunks(max_cols).enumerate() {
                let row_w: f64 = row.iter().map(|(_, rx, _)| rx * 2.0).sum::<f64>()
                    + (row.len().saturating_sub(1)) as f64 * ATTR_GAP_X;
                let row_h: f64 = row.iter().map(|(_, _, ry)| ry * 2.0).fold(0.0, f64::max);
                let row_center_y = match side {
                    Side::Top => {
                        owner.y
                            - ATTR_SIDE_GAP
                            - row_h / 2.0
                            - row_idx as f64 * (row_h + ATTR_GAP_Y)
                    }
                    Side::Bottom => {
                        owner.bottom()
                            + ATTR_SIDE_GAP
                            + row_h / 2.0
                            + row_idx as f64 * (row_h + ATTR_GAP_Y)
                    }
                    Side::Left | Side::Right => unreachable!(),
                };
                let mut x = owner.center().x - row_w / 2.0;
                for (attr, rx, ry) in row {
                    let cx = x + rx;
                    result.push(AttrLayout {
                        owner_kind,
                        owner_name: owner_name.to_string(),
                        name: attr.name.clone(),
                        kind: attr.kind,
                        cx,
                        cy: row_center_y,
                        rx: *rx,
                        ry: *ry,
                    });
                    x += rx * 2.0 + ATTR_GAP_X;
                }
            }
            result
        }
        Side::Left | Side::Right => {
            let total_h: f64 = sized_attrs.iter().map(|(_, _, ry)| ry * 2.0).sum::<f64>()
                + (sized_attrs.len().saturating_sub(1)) as f64 * ATTR_GAP_Y;
            let mut y = owner.center().y - total_h / 2.0;
            let mut result = Vec::new();
            for (attr, rx, ry) in sized_attrs {
                let cx = match side {
                    Side::Left => owner.x - ATTR_SIDE_GAP - rx,
                    Side::Right => owner.right() + ATTR_SIDE_GAP + rx,
                    Side::Top | Side::Bottom => unreachable!(),
                };
                result.push(AttrLayout {
                    owner_kind,
                    owner_name: owner_name.to_string(),
                    name: attr.name.clone(),
                    kind: attr.kind,
                    cx,
                    cy: y + ry,
                    rx,
                    ry,
                });
                y += ry * 2.0 + ATTR_GAP_Y;
            }
            result
        }
    }
}

fn placement_score(attrs: &[AttrLayout], occupied: &[Rect], preference_idx: usize) -> f64 {
    let mut score = preference_idx as f64 * 1_000.0;

    for (idx, attr) in attrs.iter().enumerate() {
        let bounds = attr.bounds();
        for other in occupied {
            if rects_overlap(bounds, *other, COLLISION_CLEARANCE) {
                score += 100_000.0 + overlap_area(bounds, *other);
            }
        }
        for previous in attrs.iter().take(idx) {
            if rects_overlap(bounds, previous.bounds(), COLLISION_CLEARANCE) {
                score += 50_000.0 + overlap_area(bounds, previous.bounds());
            }
        }
    }

    score
}

fn place_cardinality_labels(layout: &ChenLayout) -> Vec<CardinalityLayout> {
    let mut occupied = primary_bounds(layout);
    occupied.extend(
        layout
            .entities
            .iter()
            .flat_map(|entity| entity.attr_positions.iter().map(AttrLayout::bounds)),
    );
    occupied.extend(
        layout
            .relationships
            .iter()
            .flat_map(|rel| rel.attr_positions.iter().map(AttrLayout::bounds)),
    );

    let entity_by_name: BTreeMap<&str, &EntityLayout> = layout
        .entities
        .iter()
        .map(|entity| (entity.name.as_str(), entity))
        .collect();
    let mut labels = Vec::new();

    for rel in &layout.relationships {
        for (entity_name, cardinality) in &rel.participants {
            if cardinality.is_empty() {
                continue;
            }
            let Some(entity) = entity_by_name.get(entity_name.as_str()) else {
                continue;
            };

            let (ux, uy) = unit_vector(rel.cx, rel.cy, entity.cx, entity.cy);
            let normal = (-uy, ux);
            let base = Pt::new(
                rel.cx + (entity.cx - rel.cx) * 0.72,
                rel.cy + (entity.cy - rel.cy) * 0.72,
            );
            let w = estimate_text_width(cardinality) + 10.0;
            let h = 16.0;

            let candidates = [
                Pt::new(base.x + normal.0 * 18.0, base.y + normal.1 * 18.0),
                Pt::new(base.x - normal.0 * 18.0, base.y - normal.1 * 18.0),
                Pt::new(base.x + normal.0 * 30.0, base.y + normal.1 * 30.0),
                Pt::new(base.x - normal.0 * 30.0, base.y - normal.1 * 30.0),
                Pt::new(base.x, base.y - 18.0),
                Pt::new(base.x, base.y + 18.0),
            ];

            let mut best = candidates[0];
            let mut best_score = f64::INFINITY;
            for (idx, candidate) in candidates.iter().enumerate() {
                let bounds = Rect::from_center(candidate.x, candidate.y, w, h);
                let mut score = idx as f64 * 100.0;
                for other in &occupied {
                    if rects_overlap(bounds, *other, LABEL_CLEARANCE) {
                        score += 100_000.0 + overlap_area(bounds, *other);
                    }
                }
                if score < best_score {
                    best = *candidate;
                    best_score = score;
                }
            }

            let label = CardinalityLayout {
                rel_name: rel.name.clone(),
                entity_name: entity_name.clone(),
                label: cardinality.clone(),
                cx: best.x,
                cy: best.y,
                w,
                h,
            };
            occupied.push(label.bounds());
            labels.push(label);
        }
    }

    labels
}

// ─── SVG rendering helpers ────────────────────────────────────────────────────

fn shifted_bbox(rect: Rect, dx: f64, dy: f64) -> String {
    SceneRect::new(rect.x + dx, rect.y + dy, rect.w, rect.h).as_puml_bbox()
}

fn text_bbox(x: f64, y: f64, text: &str, font_size: f64) -> String {
    estimate_text_bbox(x, y, text, font_size, true).as_puml_bbox()
}

fn render_relationship_lines(out: &mut String, layout: &ChenLayout, dx: f64, dy: f64) {
    let entity_by_name: BTreeMap<&str, &EntityLayout> = layout
        .entities
        .iter()
        .map(|entity| (entity.name.as_str(), entity))
        .collect();

    for rel in &layout.relationships {
        for (entity_name, _) in &rel.participants {
            let Some(entity) = entity_by_name.get(entity_name.as_str()) else {
                continue;
            };
            let (ex, ey) = clip_to_rect_edge(
                entity.cx + dx,
                entity.cy + dy,
                entity.w / 2.0,
                entity.h / 2.0,
                rel.cx + dx,
                rel.cy + dy,
            );
            let (rx, ry) = clip_to_diamond_edge(
                rel.cx + dx,
                rel.cy + dy,
                rel.half_w,
                rel.half_h,
                entity.cx + dx,
                entity.cy + dy,
            );
            out.push_str(&format!(
                "<line class=\"chen-relationship-line puml-edge\" data-chen-relationship=\"{}\" data-chen-entity=\"{}\" data-puml-edge-id=\"relationship:{}:{}\" data-puml-from=\"{}\" data-puml-to=\"{}\" data-puml-family=\"chen\" data-puml-edge-kind=\"relationship\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"1.5\"/>\n",
                escape_text(&rel.name),
                escape_text(entity_name),
                escape_text(&rel.name),
                escape_text(entity_name),
                escape_text(entity_name),
                escape_text(&rel.name),
                ex,
                ey,
                rx,
                ry,
                CONN_COLOR
            ));
        }
    }
}

fn render_attribute_lines(out: &mut String, layout: &ChenLayout, dx: f64, dy: f64) {
    for entity in &layout.entities {
        let top = entity.cy - entity.h / 2.0;
        let bottom = entity.cy + entity.h / 2.0;
        let nearest_above_y = entity
            .attr_positions
            .iter()
            .filter(|attr| attr.cy + attr.ry <= top)
            .map(|attr| attr.cy)
            .fold(f64::NEG_INFINITY, f64::max);
        let nearest_below_y = entity
            .attr_positions
            .iter()
            .filter(|attr| attr.cy - attr.ry >= bottom)
            .map(|attr| attr.cy)
            .fold(f64::INFINITY, f64::min);
        let sibling_left = entity
            .attr_positions
            .iter()
            .map(|attr| attr.cx - attr.rx)
            .fold(f64::INFINITY, f64::min);
        let sibling_right = entity
            .attr_positions
            .iter()
            .map(|attr| attr.cx + attr.rx)
            .fold(f64::NEG_INFINITY, f64::max);
        for attr in &entity.attr_positions {
            out.push_str(&entity_attribute_line(
                entity,
                attr,
                nearest_above_y,
                nearest_below_y,
                sibling_left,
                sibling_right,
                dx,
                dy,
            ));
        }
    }

    for rel in &layout.relationships {
        for attr in &rel.attr_positions {
            let (rx, ry) = clip_to_diamond_edge(
                rel.cx + dx,
                rel.cy + dy,
                rel.half_w,
                rel.half_h,
                attr.cx + dx,
                attr.cy + dy,
            );
            let (ax, ay) = clip_to_oval_edge(
                attr.cx + dx,
                attr.cy + dy,
                attr.rx,
                attr.ry,
                rel.cx + dx,
                rel.cy + dy,
            );
            out.push_str(&attribute_line(rel.name.as_str(), attr, rx, ry, ax, ay));
        }
    }
}

fn entity_attribute_line(
    entity: &EntityLayout,
    attr: &AttrLayout,
    nearest_above_y: f64,
    nearest_below_y: f64,
    sibling_left: f64,
    sibling_right: f64,
    dx: f64,
    dy: f64,
) -> String {
    let left = entity.cx - entity.w / 2.0 + dx;
    let right = entity.cx + entity.w / 2.0 + dx;
    let top = entity.cy - entity.h / 2.0 + dy;
    let bottom = entity.cy + entity.h / 2.0 + dy;
    let attr_cx = attr.cx + dx;
    let attr_cy = attr.cy + dy;
    let attr_left = attr_cx - attr.rx;
    let attr_right = attr_cx + attr.rx;
    let above = attr.cy + attr.ry <= entity.cy - entity.h / 2.0;
    let below = attr.cy - attr.ry >= entity.cy + entity.h / 2.0;
    let stacked_above = above && attr.cy < nearest_above_y - 1.0;
    let stacked_below = below && attr.cy > nearest_below_y + 1.0;
    let outside_left = attr_cx < left;
    let outside_right = attr_cx > right;

    if outside_left || outside_right || stacked_above || stacked_below {
        let route_right = if outside_right {
            true
        } else if outside_left {
            false
        } else {
            attr_cx >= entity.cx + dx
        };
        let source_y = if attr_cy < top {
            top
        } else if attr_cy > bottom {
            bottom
        } else {
            attr_cy.clamp(top, bottom)
        };
        let source = if route_right {
            (right, source_y)
        } else {
            (left, source_y)
        };
        let target = if route_right {
            (attr_right, attr_cy)
        } else {
            (attr_left, attr_cy)
        };
        let lane_x = if route_right {
            right.max(sibling_right + dx) + 6.0
        } else {
            left.min(sibling_left + dx) - 6.0
        };
        return attribute_polyline(
            entity.name.as_str(),
            attr,
            &[source, (lane_x, source.1), (lane_x, target.1), target],
        );
    }

    let (ex, ey) = if above {
        (attr_cx.clamp(left, right), top)
    } else if below {
        (attr_cx.clamp(left, right), bottom)
    } else {
        clip_to_rect_edge(
            entity.cx + dx,
            entity.cy + dy,
            entity.w / 2.0,
            entity.h / 2.0,
            attr_cx,
            attr_cy,
        )
    };
    let (ax, ay) = clip_to_oval_edge(attr_cx, attr_cy, attr.rx, attr.ry, ex, ey);
    attribute_line(entity.name.as_str(), attr, ex, ey, ax, ay)
}

fn attribute_line(owner: &str, attr: &AttrLayout, x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    let attr_id = attr.id();
    format!(
        "<line class=\"chen-attribute-line puml-edge\" data-chen-owner=\"{}\" data-chen-attribute=\"{}\" data-puml-edge-id=\"attribute:{}\" data-puml-from=\"{}\" data-puml-to=\"{}\" data-puml-family=\"chen\" data-puml-edge-kind=\"attribute\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"1.0\"/>\n",
        escape_text(owner),
        escape_text(&attr.name),
        escape_text(&attr_id),
        escape_text(owner),
        escape_text(&attr_id),
        x1,
        y1,
        x2,
        y2,
        ATTR_LINE_COLOR
    )
}

fn attribute_polyline(owner: &str, attr: &AttrLayout, points: &[(f64, f64)]) -> String {
    let attr_id = attr.id();
    let points = points
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "<polyline class=\"chen-attribute-line puml-edge\" data-chen-owner=\"{}\" data-chen-attribute=\"{}\" data-puml-edge-id=\"attribute:{}\" data-puml-from=\"{}\" data-puml-to=\"{}\" data-puml-family=\"chen\" data-puml-edge-kind=\"attribute\" points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.0\"/>\n",
        escape_text(owner),
        escape_text(&attr.name),
        escape_text(&attr_id),
        escape_text(owner),
        escape_text(&attr_id),
        points,
        ATTR_LINE_COLOR
    )
}

fn render_entity(entity: &EntityLayout, dx: f64, dy: f64) -> String {
    let x = entity.cx - entity.w / 2.0 + dx;
    let y = entity.cy - entity.h / 2.0 + dy;
    let mut s = String::new();

    if entity.is_weak {
        s.push_str(&format!(
            "<rect class=\"chen-entity-outline\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"white\" stroke=\"{}\" stroke-width=\"3.5\"/>\n",
            x - 4.0,
            y - 4.0,
            entity.w + 8.0,
            entity.h + 8.0,
            WEAK_STROKE
        ));
    }
    let node_bbox = shifted_bbox(entity.bounds(), dx, dy);
    s.push_str(&format!(
        "<rect class=\"chen-entity puml-node\" data-chen-entity=\"{}\" data-puml-id=\"{}\" data-puml-kind=\"entity\" data-puml-family=\"chen\" data-puml-bbox=\"{}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>\n",
        escape_text(&entity.name),
        escape_text(&entity.name),
        node_bbox,
        x,
        y,
        entity.w,
        entity.h,
        ENTITY_FILL,
        ENTITY_STROKE
    ));
    let label_bbox = text_bbox(entity.cx + dx, entity.cy + dy, &entity.name, 13.0);
    s.push_str(&format!(
        "<text class=\"chen-entity-label puml-label\" data-puml-owner=\"{}\" data-puml-label-kind=\"node-label\" data-puml-bbox=\"{}\" x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>\n",
        escape_text(&entity.name),
        label_bbox,
        entity.cx + dx,
        entity.cy + dy,
        FONT_ATTRS,
        ENTITY_TEXT,
        escape_text(&entity.name)
    ));
    s
}

fn render_relationship(rel: &RelLayout, dx: f64, dy: f64) -> String {
    let cx = rel.cx + dx;
    let cy = rel.cy + dy;
    let points = diamond_points(cx, cy, rel.half_w, rel.half_h);
    let mut s = String::new();

    if rel.is_identifying {
        s.push_str(&format!(
            "<polygon class=\"chen-relationship-outline\" points=\"{}\" fill=\"white\" stroke=\"{}\" stroke-width=\"2\"/>\n",
            diamond_points(cx, cy, rel.half_w + 5.0, rel.half_h + 5.0),
            REL_STROKE
        ));
    }
    let node_bbox = shifted_bbox(rel.bounds(), dx, dy);
    s.push_str(&format!(
        "<polygon class=\"chen-relationship puml-node\" data-chen-relationship=\"{}\" data-puml-id=\"{}\" data-puml-kind=\"relationship\" data-puml-family=\"chen\" data-puml-bbox=\"{}\" points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>\n",
        escape_text(&rel.name),
        escape_text(&rel.name),
        node_bbox,
        points,
        REL_FILL,
        REL_STROKE
    ));
    let label_bbox = text_bbox(cx, cy, &rel.name, 13.0);
    s.push_str(&format!(
        "<text class=\"chen-relationship-label puml-label\" data-puml-owner=\"{}\" data-puml-label-kind=\"node-label\" data-puml-bbox=\"{}\" x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>\n",
        escape_text(&rel.name),
        label_bbox,
        cx,
        cy,
        FONT_ATTRS,
        REL_TEXT,
        escape_text(&rel.name)
    ));
    s
}

fn render_attribute_oval(attr: &AttrLayout, dx: f64, dy: f64) -> String {
    let cx = attr.cx + dx;
    let cy = attr.cy + dy;
    let mut s = String::new();
    let id = attr.id();

    if attr.kind == ChenAttrKind::Multivalued {
        s.push_str(&format!(
            "<ellipse class=\"chen-attribute-outline\" cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>\n",
            cx,
            cy,
            attr.rx + 4.0,
            attr.ry + 4.0,
            ATTR_STROKE
        ));
    }

    let stroke_dash = if attr.kind == ChenAttrKind::Derived {
        " stroke-dasharray=\"4 3\""
    } else {
        ""
    };
    s.push_str(&format!(
        "<ellipse class=\"chen-attribute puml-node\" data-chen-owner-kind=\"{}\" data-chen-owner=\"{}\" data-chen-attribute=\"{}\" data-chen-attribute-id=\"{}\" data-puml-id=\"{}\" data-puml-kind=\"attribute\" data-puml-family=\"chen\" data-puml-bbox=\"{}\" cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>\n",
        attr.owner_kind,
        escape_text(&attr.owner_name),
        escape_text(&attr.name),
        escape_text(&id),
        escape_text(&id),
        shifted_bbox(attr.bounds(), dx, dy),
        cx,
        cy,
        attr.rx,
        attr.ry,
        ATTR_FILL,
        ATTR_STROKE,
        stroke_dash
    ));

    let decoration = if attr.kind == ChenAttrKind::Key {
        " text-decoration=\"underline\""
    } else {
        ""
    };
    let label_bbox = text_bbox(cx, cy, &attr.name, 13.0);
    s.push_str(&format!(
        "<text class=\"chen-attribute-label puml-label\" data-puml-owner=\"{}\" data-puml-label-kind=\"attribute-label\" data-puml-bbox=\"{}\" x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\"{}>{}</text>\n",
        escape_text(&id),
        label_bbox,
        cx,
        cy,
        FONT_ATTRS,
        ATTR_TEXT,
        decoration,
        escape_text(&attr.name)
    ));
    s
}

fn render_cardinality_label(label: &CardinalityLayout, dx: f64, dy: f64) -> String {
    let owner = format!("{}:{}", label.rel_name, label.entity_name);
    let bbox = shifted_bbox(label.bounds(), dx, dy);
    format!(
        "<text class=\"chen-cardinality puml-label\" data-chen-relationship=\"{}\" data-chen-entity=\"{}\" data-puml-owner=\"{}\" data-puml-label-kind=\"cardinality\" data-puml-bbox=\"{}\" x=\"{:.1}\" y=\"{:.1}\" {} fill=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>\n",
        escape_text(&label.rel_name),
        escape_text(&label.entity_name),
        escape_text(&owner),
        bbox,
        label.cx + dx,
        label.cy + dy,
        FONT_ATTRS,
        CONN_COLOR,
        escape_text(&label.label)
    )
}

// ─── Geometry helpers ─────────────────────────────────────────────────────────

fn primary_bounds(layout: &ChenLayout) -> Vec<Rect> {
    layout
        .entities
        .iter()
        .map(EntityLayout::bounds)
        .chain(layout.relationships.iter().map(RelLayout::bounds))
        .collect()
}

fn canvas_transform(
    layout: &ChenLayout,
    cardinalities: &[CardinalityLayout],
    has_title: bool,
) -> (f64, f64, f64, f64) {
    let mut bounds = primary_bounds(layout);
    bounds.extend(
        layout
            .entities
            .iter()
            .flat_map(|entity| entity.attr_positions.iter().map(AttrLayout::bounds)),
    );
    bounds.extend(
        layout
            .relationships
            .iter()
            .flat_map(|rel| rel.attr_positions.iter().map(AttrLayout::bounds)),
    );
    bounds.extend(cardinalities.iter().map(CardinalityLayout::bounds));

    let min_x = bounds.iter().map(|b| b.x).fold(0.0, f64::min);
    let min_y = bounds.iter().map(|b| b.y).fold(0.0, f64::min);
    let max_x = bounds.iter().map(|b| b.right()).fold(160.0, f64::max);
    let max_y = bounds.iter().map(|b| b.bottom()).fold(120.0, f64::max);

    let title_h = if has_title { TITLE_H } else { 0.0 };
    let shift_x = CONTENT_MARGIN - min_x;
    let shift_y = title_h + CONTENT_MARGIN - min_y;
    let width = (max_x + shift_x + CONTENT_MARGIN).max(240.0);
    let height = (max_y + shift_y + CONTENT_MARGIN).max(title_h + 160.0);
    (shift_x, shift_y, width, height)
}

fn entity_size(name: &str) -> (f64, f64) {
    (
        (estimate_text_width(name) + ENTITY_PAD_X * 2.0).max(ENTITY_MIN_W),
        ENTITY_H,
    )
}

fn relationship_size(name: &str) -> (f64, f64) {
    (
        (estimate_text_width(name) / 2.0 + DIAMOND_PAD_X).max(DIAMOND_MIN_HALF_W),
        DIAMOND_HALF_H,
    )
}

fn attribute_radii(name: &str) -> (f64, f64) {
    (
        ((estimate_text_width(name) + ATTR_PAD_X * 2.0).max(ATTR_MIN_W)) / 2.0,
        ATTR_RY,
    )
}

fn estimate_text_width(text: &str) -> f64 {
    text.chars().count() as f64 * 7.0
}

fn entity_id(idx: usize, name: &str) -> String {
    format!("entity:{idx:04}:{name}")
}

fn rel_id(idx: usize, name: &str) -> String {
    format!("relationship:{idx}:{name}")
}

fn rects_overlap(a: Rect, b: Rect, clearance: f64) -> bool {
    a.x < b.right() + clearance
        && a.right() + clearance > b.x
        && a.y < b.bottom() + clearance
        && a.bottom() + clearance > b.y
}

fn overlap_area(a: Rect, b: Rect) -> f64 {
    let x_overlap = (a.right().min(b.right()) - a.x.max(b.x)).max(0.0);
    let y_overlap = (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0.0);
    x_overlap * y_overlap
}

fn unit_vector(x1: f64, y1: f64, x2: f64, y2: f64) -> (f64, f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    (dx / len, dy / len)
}

fn diamond_points(cx: f64, cy: f64, half_w: f64, half_h: f64) -> String {
    format!(
        "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
        cx,
        cy - half_h,
        cx + half_w,
        cy,
        cx,
        cy + half_h,
        cx - half_w,
        cy
    )
}

fn clip_to_rect_edge(cx: f64, cy: f64, half_w: f64, half_h: f64, tx: f64, ty: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
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
    (cx + t * dx, cy + t * dy)
}

fn clip_to_diamond_edge(
    cx: f64,
    cy: f64,
    half_w: f64,
    half_h: f64,
    tx: f64,
    ty: f64,
) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    let denom = dx.abs() / half_w + dy.abs() / half_h;
    if denom < 1e-9 {
        return (cx, cy);
    }
    let t = 1.0 / denom;
    (cx + t * dx, cy + t * dy)
}

fn clip_to_oval_edge(cx: f64, cy: f64, rx: f64, ry: f64, tx: f64, ty: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    let t = 1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt().max(1e-9);
    (cx + t * dx, cy + t * dy)
}

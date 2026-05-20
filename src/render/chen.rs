mod drawing;

use drawing::*;
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

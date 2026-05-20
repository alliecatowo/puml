use super::*;

pub(super) fn shifted_bbox(rect: Rect, dx: f64, dy: f64) -> String {
    SceneRect::new(rect.x + dx, rect.y + dy, rect.w, rect.h).as_puml_bbox()
}

pub(super) fn text_bbox(x: f64, y: f64, text: &str, font_size: f64) -> String {
    estimate_text_bbox(x, y, text, font_size, true).as_puml_bbox()
}

pub(super) fn render_relationship_lines(out: &mut String, layout: &ChenLayout, dx: f64, dy: f64) {
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

pub(super) fn render_attribute_lines(out: &mut String, layout: &ChenLayout, dx: f64, dy: f64) {
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
        let routing = EntityAttributeRouting {
            nearest_above_y,
            nearest_below_y,
            sibling_left,
            sibling_right,
            dx,
            dy,
        };
        for attr in &entity.attr_positions {
            out.push_str(&entity_attribute_line(entity, attr, routing));
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

#[derive(Debug, Clone, Copy)]
pub(super) struct EntityAttributeRouting {
    nearest_above_y: f64,
    nearest_below_y: f64,
    sibling_left: f64,
    sibling_right: f64,
    dx: f64,
    dy: f64,
}

pub(super) fn entity_attribute_line(
    entity: &EntityLayout,
    attr: &AttrLayout,
    routing: EntityAttributeRouting,
) -> String {
    let left = entity.cx - entity.w / 2.0 + routing.dx;
    let right = entity.cx + entity.w / 2.0 + routing.dx;
    let top = entity.cy - entity.h / 2.0 + routing.dy;
    let bottom = entity.cy + entity.h / 2.0 + routing.dy;
    let attr_cx = attr.cx + routing.dx;
    let attr_cy = attr.cy + routing.dy;
    let attr_left = attr_cx - attr.rx;
    let attr_right = attr_cx + attr.rx;
    let above = attr.cy + attr.ry <= entity.cy - entity.h / 2.0;
    let below = attr.cy - attr.ry >= entity.cy + entity.h / 2.0;
    let stacked_above = above && attr.cy < routing.nearest_above_y - 1.0;
    let stacked_below = below && attr.cy > routing.nearest_below_y + 1.0;
    let outside_left = attr_cx < left;
    let outside_right = attr_cx > right;

    if outside_left || outside_right || stacked_above || stacked_below {
        let route_right = if outside_right {
            true
        } else if outside_left {
            false
        } else {
            attr_cx >= entity.cx + routing.dx
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
            right.max(routing.sibling_right + routing.dx) + 6.0
        } else {
            left.min(routing.sibling_left + routing.dx) - 6.0
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
            entity.cx + routing.dx,
            entity.cy + routing.dy,
            entity.w / 2.0,
            entity.h / 2.0,
            attr_cx,
            attr_cy,
        )
    };
    let (ax, ay) = clip_to_oval_edge(attr_cx, attr_cy, attr.rx, attr.ry, ex, ey);
    attribute_line(entity.name.as_str(), attr, ex, ey, ax, ay)
}

pub(super) fn attribute_line(
    owner: &str,
    attr: &AttrLayout,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> String {
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

pub(super) fn attribute_polyline(owner: &str, attr: &AttrLayout, points: &[(f64, f64)]) -> String {
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

pub(super) fn render_entity(entity: &EntityLayout, dx: f64, dy: f64) -> String {
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

pub(super) fn render_relationship(rel: &RelLayout, dx: f64, dy: f64) -> String {
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

pub(super) fn render_attribute_oval(attr: &AttrLayout, dx: f64, dy: f64) -> String {
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

pub(super) fn render_cardinality_label(label: &CardinalityLayout, dx: f64, dy: f64) -> String {
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

pub(super) fn primary_bounds(layout: &ChenLayout) -> Vec<Rect> {
    layout
        .entities
        .iter()
        .map(EntityLayout::bounds)
        .chain(layout.relationships.iter().map(RelLayout::bounds))
        .collect()
}

pub(super) fn canvas_transform(
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

pub(super) fn entity_size(name: &str) -> (f64, f64) {
    (
        (estimate_text_width(name) + ENTITY_PAD_X * 2.0).max(ENTITY_MIN_W),
        ENTITY_H,
    )
}

pub(super) fn relationship_size(name: &str) -> (f64, f64) {
    (
        (estimate_text_width(name) / 2.0 + DIAMOND_PAD_X).max(DIAMOND_MIN_HALF_W),
        DIAMOND_HALF_H,
    )
}

pub(super) fn attribute_radii(name: &str) -> (f64, f64) {
    (
        ((estimate_text_width(name) + ATTR_PAD_X * 2.0).max(ATTR_MIN_W)) / 2.0,
        ATTR_RY,
    )
}

pub(super) fn estimate_text_width(text: &str) -> f64 {
    text.chars().count() as f64 * 7.0
}

pub(super) fn entity_id(idx: usize, name: &str) -> String {
    format!("entity:{idx:04}:{name}")
}

pub(super) fn rel_id(idx: usize, name: &str) -> String {
    format!("relationship:{idx}:{name}")
}

pub(super) fn rects_overlap(a: Rect, b: Rect, clearance: f64) -> bool {
    a.x < b.right() + clearance
        && a.right() + clearance > b.x
        && a.y < b.bottom() + clearance
        && a.bottom() + clearance > b.y
}

pub(super) fn overlap_area(a: Rect, b: Rect) -> f64 {
    let x_overlap = (a.right().min(b.right()) - a.x.max(b.x)).max(0.0);
    let y_overlap = (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0.0);
    x_overlap * y_overlap
}

pub(super) fn unit_vector(x1: f64, y1: f64, x2: f64, y2: f64) -> (f64, f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    (dx / len, dy / len)
}

pub(super) fn diamond_points(cx: f64, cy: f64, half_w: f64, half_h: f64) -> String {
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

pub(super) fn clip_to_rect_edge(
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

pub(super) fn clip_to_diamond_edge(
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

pub(super) fn clip_to_oval_edge(
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    tx: f64,
    ty: f64,
) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    let t = 1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt().max(1e-9);
    (cx + t * dx, cy + t * dy)
}

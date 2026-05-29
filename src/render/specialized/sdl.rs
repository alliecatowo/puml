use super::*;

/// Compute BFS-based rank (depth from source) for each SDL state.
///
/// States reachable from the first `Start` node receive their BFS depth as rank.
/// Unreachable states are placed after all reachable ones at rank = max_rank + 1.
/// Back-edges (cycles, e.g. retry loops) are skipped so ranks are finite.
fn sdl_compute_ranks(document: &SdlDocument) -> BTreeMap<&str, usize> {
    use std::collections::VecDeque;
    // Build adjacency: from → Vec<to>
    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for s in &document.states {
        adj.entry(s.name.as_str()).or_default();
    }
    for tr in &document.transitions {
        adj.entry(tr.from.as_str())
            .or_default()
            .push(tr.to.as_str());
    }

    let mut ranks: BTreeMap<&str, usize> = BTreeMap::new();
    // Find the Start node (or fall back to first state)
    let start = document
        .states
        .iter()
        .find(|s| s.kind == SdlStateKind::Start)
        .or_else(|| document.states.first())
        .map(|s| s.name.as_str());

    if let Some(root) = start {
        let mut queue: VecDeque<&str> = VecDeque::new();
        ranks.insert(root, 0);
        queue.push_back(root);
        while let Some(node) = queue.pop_front() {
            let r = *ranks.get(node).unwrap_or(&0);
            for &neighbour in adj.get(node).map(|v| v.as_slice()).unwrap_or(&[]) {
                if !ranks.contains_key(neighbour) {
                    ranks.insert(neighbour, r + 1);
                    queue.push_back(neighbour);
                }
            }
        }
    }

    // Unreachable nodes get rank = max_rank + 1
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    for s in &document.states {
        ranks.entry(s.name.as_str()).or_insert(max_rank + 1);
    }
    ranks
}

pub fn render_sdl_svg(document: &SdlDocument) -> String {
    render_sdl_artifact(document).svg
}

/// Render an SDL diagram into a typed [`RenderArtifact`].
///
/// The SVG is emitted directly from the laid-out grid geometry. We also build a
/// [`RenderScene`] from the *same* positions and transition endpoints the SVG uses,
/// so the scene stays consistent with the rendered output. SVG output is
/// byte-identical to the legacy `render_sdl_svg`; the scene is attached for the
/// typed-geometry validation path.
pub fn render_sdl_artifact(document: &SdlDocument) -> RenderArtifact {
    let col_w = 260;
    let row_h = 96;
    let margin_x = 40;
    let header_h = if document.title.is_some() { 64 } else { 40 };

    // Compute topology-aware ranks so nodes are positioned in BFS order.
    let ranks = sdl_compute_ranks(document);

    // Group states by rank, preserving document order within each rank.
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    let mut by_rank: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for s in &document.states {
        by_rank
            .entry(*ranks.get(s.name.as_str()).unwrap_or(&0))
            .or_default()
            .push(s.name.as_str());
    }
    let max_cols = by_rank.values().map(|v| v.len()).max().unwrap_or(1) as i32;
    let rows = (max_rank + 1) as i32;
    let width = margin_x * 2 + max_cols * col_w;
    let height = header_h + rows * row_h + 48;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    // refX="9" places the arrowhead *tip* (at marker x=9) exactly at the line
    // endpoint, so the arrowhead base sits outside the target node and the
    // triangle points unambiguously toward its target.
    out.push_str(
        "<defs><marker id=\"sdl-arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"9\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L9,3 z\" fill=\"#334155\"/></marker></defs>",
    );
    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" fill=\"#0f172a\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">SDL diagram</text>"
    ));
    let grid_top = header_h;

    // Build position map using topology-aware rank + column slot.
    let mut positions: BTreeMap<&str, SdlNodeBox> = BTreeMap::new();
    // Look up state kind by name for sdl_node_box.
    let kind_by_name: BTreeMap<&str, SdlStateKind> = document
        .states
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    for (rank, names) in &by_rank {
        let n = names.len() as i32;
        // Centre the nodes within this rank horizontally.
        let rank_width = n * col_w;
        let start_x = margin_x + (width - margin_x * 2 - rank_width) / 2;
        for (col, &name) in names.iter().enumerate() {
            let kind = kind_by_name
                .get(name)
                .copied()
                .unwrap_or(SdlStateKind::State);
            let x = start_x + col as i32 * col_w + (col_w - SDL_NODE_W) / 2;
            let row_y = grid_top + *rank as i32 * row_h + 12;
            let node = sdl_node_box(x, row_y, kind);
            positions.insert(name, node);
        }
    }

    let mut transition_labels = String::new();
    for tr in &document.transitions {
        let Some(from) = positions.get(tr.from.as_str()) else {
            continue;
        };
        let Some(to) = positions.get(tr.to.as_str()) else {
            continue;
        };
        render_sdl_transition(&mut out, &mut transition_labels, tr, *from, *to);
    }

    for state in &document.states {
        if let Some(node) = positions.get(state.name.as_str()) {
            render_sdl_node(&mut out, state, *node);
        }
    }
    out.push_str(&transition_labels);
    out.push_str("</svg>");

    let scene = super::sdl_scene::build_sdl_scene(
        &positions,
        &document.transitions,
        width as f64,
        height as f64,
    );
    RenderArtifact::with_scene(out, scene)
}

pub(super) const SDL_NODE_W: i32 = 168;
const SDL_NODE_H: i32 = 48;

#[derive(Debug, Clone, Copy)]
pub(super) struct SdlNodeBox {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) kind: SdlStateKind,
}

fn sdl_node_box(x: i32, y: i32, kind: SdlStateKind) -> SdlNodeBox {
    match kind {
        SdlStateKind::Start | SdlStateKind::Stop => SdlNodeBox {
            x: x + 44,
            y,
            w: 80,
            h: 56,
            kind,
        },
        SdlStateKind::Decision => SdlNodeBox {
            x: x + 12,
            y: y - 8,
            w: 144,
            h: 72,
            kind,
        },
        SdlStateKind::Input | SdlStateKind::Output | SdlStateKind::State => SdlNodeBox {
            x,
            y,
            w: SDL_NODE_W,
            h: SDL_NODE_H,
            kind,
        },
    }
}

fn render_sdl_transition(
    out: &mut String,
    labels_out: &mut String,
    tr: &crate::model::SdlTransition,
    from: SdlNodeBox,
    to: SdlNodeBox,
) {
    let (x1, y1, x2, y2) = sdl_transition_endpoints(from, to);
    // True self-loop: same node position.
    let is_self_loop = from.x == to.x && from.y == to.y;
    // Back-edge: target is above the source (y2 < y1 by a significant margin).
    // These need curved routing so the arrow doesn't pass through intermediate nodes.
    let is_back_edge = !is_self_loop && y2 < y1 - 10;
    if is_self_loop {
        let cx = from.x + from.w;
        let cy = from.y + from.h / 2;
        out.push_str(&format!(
            "<path class=\"sdl-transition\" data-sdl-from=\"{}\" data-sdl-to=\"{}\" d=\"M {cx} {cy} C {} {}, {} {}, {cx} {}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" marker-end=\"url(#sdl-arrow)\"/>",
            escape_text(&tr.from),
            escape_text(&tr.to),
            cx + 46,
            cy - 24,
            cx + 46,
            cy + 34,
            cy + 10,
        ));
    } else if is_back_edge {
        // Route back-edges as a wide curved arc to the LEFT of the node column
        // so the arc clearly bypasses intermediate nodes and avoids text overlap.
        // Push control points far left (-120px) so the curve stays visually
        // distinct from the forward arrows and any node boundaries.
        let offset = -120;
        let cp1x = x1 + offset;
        let cp2x = x2 + offset;
        out.push_str(&format!(
            "<path class=\"sdl-transition\" data-sdl-from=\"{}\" data-sdl-to=\"{}\" d=\"M {x1} {y1} C {cp1x} {y1}, {cp2x} {y2}, {x2} {y2}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" marker-end=\"url(#sdl-arrow)\"/>",
            escape_text(&tr.from),
            escape_text(&tr.to),
        ));
    } else {
        out.push_str(&format!(
            "<line class=\"sdl-transition\" data-sdl-from=\"{}\" data-sdl-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#334155\" stroke-width=\"1.5\" marker-end=\"url(#sdl-arrow)\"/>",
            escape_text(&tr.from),
            escape_text(&tr.to),
        ));
    }
    if let Some(label) = &tr.signal {
        // Compute label position at the path midpoint with a perpendicular
        // offset so the text sits clearly beside the shaft rather than on it.
        let (lx, ly) = if is_self_loop {
            // Self-loop: cubic bezier bulges to the right (+x).  Place the
            // label at the loop apex — cx+46 at cy, offset slightly right so
            // it clears the loop arc.
            let cx = from.x + from.w;
            let cy = from.y + from.h / 2;
            (cx + 52, cy + 5)
        } else if is_back_edge {
            // Back-edge: cubic S-curve to the left.  Place label at the
            // leftmost extent of the curve (the control-point level midpoint).
            let mid_y = (y1 + y2) / 2;
            let curve_left_x = x1 - 120; // matches the offset above
            (curve_left_x - 5, mid_y)
        } else {
            // Straight line: midpoint + perpendicular offset of 10 px.
            // The perpendicular (left of the direction of travel) is:
            //   perp = (-dy, dx) / ||(dx,dy)||
            // We always offset to the "left" side (counter-clockwise from the
            // travel direction) which gives a consistent visual placement.
            let dx = (x2 - x1) as f64;
            let dy = (y2 - y1) as f64;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            // Left-perpendicular unit vector: (-dy/len, dx/len).
            let perp_x = -dy / len;
            let perp_y = dx / len;
            let offset = 10.0_f64;
            let mx = (x1 + x2) as f64 / 2.0 + perp_x * offset;
            let my = (y1 + y2) as f64 / 2.0 + perp_y * offset;
            (mx.round() as i32, my.round() as i32)
        };
        // Approximate half-width of the label text so we can draw a white
        // background rect that prevents any arrow from bleeding through.
        // Use the shared text-width helper (7 px/char at 14 px font).
        let approx_half_w =
            crate::render_core::text_metrics::estimate_text_width_default(label) / 2 + 4;
        let font_h: i32 = 11;
        let pad: i32 = 2;
        labels_out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\"/>",
            lx - approx_half_w,
            ly - font_h - pad,
            approx_half_w * 2,
            font_h + pad * 2,
        ));
        labels_out.push_str(&format!(
            "<text class=\"sdl-transition-label\" x=\"{lx}\" y=\"{ly}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#475569\">{}</text>",
            escape_text(label)
        ));
    }
}

/// For circular nodes (Start/Stop) the rendered circle's centre within the
/// bounding box is at y=node.y+18 and radius 13 (Start) / 15 (Stop).  The
/// bounding box is 80 wide and 56 tall, placing significant whitespace below
/// the circle that would push a bounding-box-edge endpoint far from the actual
/// circle.  Return the actual circle centre and radius for accurate endpoint
/// computation.
fn sdl_circle_params(node: SdlNodeBox) -> (i32, i32, i32) {
    let cx = node.x + node.w / 2;
    let cy = node.y + 18;
    let r = if node.kind == SdlStateKind::Start {
        13
    } else {
        15
    };
    (cx, cy, r)
}

pub(super) fn sdl_transition_endpoints(from: SdlNodeBox, to: SdlNodeBox) -> (i32, i32, i32, i32) {
    // Centre points (used for direction computation).
    // For circular nodes (Start/Stop) use the actual circle centre (y + 18);
    // for rectangular nodes use the bounding-box centre.
    let fcx = from.x + from.w / 2;
    let fcy = if matches!(from.kind, SdlStateKind::Start | SdlStateKind::Stop) {
        from.y + 18
    } else {
        from.y + from.h / 2
    };
    let tcx = to.x + to.w / 2;
    let tcy = if matches!(to.kind, SdlStateKind::Start | SdlStateKind::Stop) {
        to.y + 18
    } else {
        to.y + to.h / 2
    };
    let dx = tcx - fcx;
    let dy = tcy - fcy;

    // Target endpoint: for circular nodes snap to the circle surface so the
    // arrowhead tip (placed exactly at the endpoint via refX=9) lands on the
    // node border.  For rectangular nodes use the axis-aligned bounding-box
    // edge aligned with the dominant direction.
    let (x2, y2) = if matches!(to.kind, SdlStateKind::Start | SdlStateKind::Stop) {
        let (cx, cy, r) = sdl_circle_params(to);
        let ddx = (cx - fcx) as f64;
        let ddy = (cy - fcy) as f64;
        let len = (ddx * ddx + ddy * ddy).sqrt().max(1.0);
        // Place endpoint on the circle surface (inward direction from centre).
        let ex = cx - (ddx / len * r as f64).round() as i32;
        let ey = cy - (ddy / len * r as f64).round() as i32;
        (ex, ey)
    } else if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (to.x, tcy)
        } else {
            (to.x + to.w, tcy)
        }
    } else if dy >= 0 {
        (tcx, to.y)
    } else {
        (tcx, to.y + to.h)
    };

    // Source endpoint: exit the from-node from its border, or the circle edge.
    let (x1, y1) = if matches!(from.kind, SdlStateKind::Start | SdlStateKind::Stop) {
        let (cx, cy, r) = sdl_circle_params(from);
        let ddx = (tcx - cx) as f64;
        let ddy = (tcy - cy) as f64;
        let len = (ddx * ddx + ddy * ddy).sqrt().max(1.0);
        let ex = cx + (ddx / len * r as f64).round() as i32;
        let ey = cy + (ddy / len * r as f64).round() as i32;
        (ex, ey)
    } else if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (from.x + from.w, fcy)
        } else {
            (from.x, fcy)
        }
    } else if dy >= 0 {
        (fcx, from.y + from.h)
    } else {
        (fcx, from.y)
    };

    (x1, y1, x2, y2)
}

fn render_sdl_node(out: &mut String, state: &crate::model::SdlState, node: SdlNodeBox) {
    let kind = sdl_state_kind_label(state.kind);
    out.push_str(&format!(
        "<g class=\"sdl-node sdl-{kind}\" data-sdl-kind=\"{kind}\" data-sdl-name=\"{}\">",
        escape_text(&state.name)
    ));
    match state.kind {
        SdlStateKind::Start => {
            let cx = node.x + node.w / 2;
            let cy = node.y + 18;
            out.push_str(&format!(
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"13\" fill=\"#111827\"/>"
            ));
            render_sdl_label(out, &state.name, cx, node.y + 50, "#111827");
        }
        SdlStateKind::Stop => {
            let cx = node.x + node.w / 2;
            let cy = node.y + 18;
            out.push_str(&format!(
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"15\" fill=\"none\" stroke=\"#111827\" stroke-width=\"2\"/><circle cx=\"{cx}\" cy=\"{cy}\" r=\"9\" fill=\"#111827\"/>"
            ));
            render_sdl_label(out, &state.name, cx, node.y + 50, "#111827");
        }
        SdlStateKind::Decision => {
            let cx = node.x + node.w / 2;
            let cy = node.y + node.h / 2;
            out.push_str(&format!(
                "<polygon points=\"{cx},{} {},{cy} {cx},{} {},{cy}\" fill=\"#fef3c7\" stroke=\"#b45309\" stroke-width=\"1.5\"/>",
                node.y,
                node.x + node.w,
                node.y + node.h,
                node.x,
            ));
            render_sdl_label(out, &state.name, cx, cy + 4, "#78350f");
        }
        SdlStateKind::Input => {
            let slant = 16;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#e0f2fe\" stroke=\"#0284c7\" stroke-width=\"1.5\"/>",
                node.x + slant,
                node.y,
                node.x + node.w,
                node.y,
                node.x + node.w - slant,
                node.y + node.h,
                node.x,
                node.y + node.h,
            ));
            render_sdl_label(
                out,
                &state.name,
                node.x + node.w / 2,
                node.y + node.h / 2 + 4,
                "#075985",
            );
        }
        SdlStateKind::Output => {
            let slant = 16;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#dcfce7\" stroke=\"#16a34a\" stroke-width=\"1.5\"/>",
                node.x,
                node.y,
                node.x + node.w - slant,
                node.y,
                node.x + node.w,
                node.y + node.h,
                node.x + slant,
                node.y + node.h,
            ));
            render_sdl_label(
                out,
                &state.name,
                node.x + node.w / 2,
                node.y + node.h / 2 + 4,
                "#166534",
            );
        }
        SdlStateKind::State => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#e0e7ff\" stroke=\"#4f46e5\" stroke-width=\"1.5\"/>",
                node.x, node.y, node.w, node.h
            ));
            render_sdl_label(
                out,
                &state.name,
                node.x + node.w / 2,
                node.y + node.h / 2 + 4,
                "#312e81",
            );
        }
    }
    out.push_str("</g>");
}

fn render_sdl_label(out: &mut String, text: &str, x: i32, y: i32, fill: &str) {
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{fill}\">{}</text>",
        escape_text(text)
    ));
}

fn sdl_state_kind_label(kind: SdlStateKind) -> &'static str {
    match kind {
        SdlStateKind::Start => "start",
        SdlStateKind::Input => "input",
        SdlStateKind::Output => "output",
        SdlStateKind::Decision => "decision",
        SdlStateKind::Stop => "stop",
        SdlStateKind::State => "state",
    }
}

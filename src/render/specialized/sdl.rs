use super::*;

pub fn render_sdl_svg(document: &SdlDocument) -> String {
    let state_count = document.states.len().max(1) as i32;
    let cols = state_count.clamp(1, 2);
    let col_w = 260;
    let row_h = 96;
    let margin_x = 40;
    let header_h = if document.title.is_some() { 64 } else { 40 };
    let rows = (state_count + cols - 1) / cols;
    let width = margin_x * 2 + cols * col_w;
    let height = header_h + rows * row_h + 48;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str(
        "<defs><marker id=\"sdl-arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L9,3 z\" fill=\"#334155\"/></marker></defs>",
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
    let mut positions: BTreeMap<&str, SdlNodeBox> = BTreeMap::new();
    for (idx, state) in document.states.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let node = sdl_node_box(
            margin_x + col * col_w + (col_w - SDL_NODE_W) / 2,
            grid_top + row * row_h + 12,
            state.kind,
        );
        positions.insert(&state.name, node);
    }

    for tr in &document.transitions {
        let Some(from) = positions.get(tr.from.as_str()) else {
            continue;
        };
        let Some(to) = positions.get(tr.to.as_str()) else {
            continue;
        };
        render_sdl_transition(&mut out, tr, *from, *to);
    }

    for state in &document.states {
        if let Some(node) = positions.get(state.name.as_str()) {
            render_sdl_node(&mut out, state, *node);
        }
    }
    out.push_str("</svg>");
    out
}

const SDL_NODE_W: i32 = 168;
const SDL_NODE_H: i32 = 48;

#[derive(Debug, Clone, Copy)]
struct SdlNodeBox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn sdl_node_box(x: i32, y: i32, kind: SdlStateKind) -> SdlNodeBox {
    match kind {
        SdlStateKind::Start | SdlStateKind::Stop => SdlNodeBox {
            x: x + 44,
            y,
            w: 80,
            h: 56,
        },
        SdlStateKind::Decision => SdlNodeBox {
            x: x + 12,
            y: y - 8,
            w: 144,
            h: 72,
        },
        SdlStateKind::Input | SdlStateKind::Output | SdlStateKind::State => SdlNodeBox {
            x,
            y,
            w: SDL_NODE_W,
            h: SDL_NODE_H,
        },
    }
}

fn render_sdl_transition(
    out: &mut String,
    tr: &crate::model::SdlTransition,
    from: SdlNodeBox,
    to: SdlNodeBox,
) {
    let (x1, y1, x2, y2) = sdl_transition_endpoints(from, to);
    if from.x == to.x && from.y == to.y {
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
    } else {
        out.push_str(&format!(
            "<line class=\"sdl-transition\" data-sdl-from=\"{}\" data-sdl-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#334155\" stroke-width=\"1.5\" marker-end=\"url(#sdl-arrow)\"/>",
            escape_text(&tr.from),
            escape_text(&tr.to),
        ));
    }
    if let Some(label) = &tr.signal {
        let lx = (x1 + x2) / 2;
        let ly = (y1 + y2) / 2 - 8;
        out.push_str(&format!(
            "<text class=\"sdl-transition-label\" x=\"{lx}\" y=\"{ly}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#475569\">{}</text>",
            escape_text(label)
        ));
    }
}

fn sdl_transition_endpoints(from: SdlNodeBox, to: SdlNodeBox) -> (i32, i32, i32, i32) {
    let fcx = from.x + from.w / 2;
    let fcy = from.y + from.h / 2;
    let tcx = to.x + to.w / 2;
    let tcy = to.y + to.h / 2;
    let dx = tcx - fcx;
    let dy = tcy - fcy;
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (from.x + from.w, fcy, to.x, tcy)
        } else {
            (from.x, fcy, to.x + to.w, tcy)
        }
    } else if dy >= 0 {
        (fcx, from.y + from.h, tcx, to.y)
    } else {
        (fcx, from.y, tcx, to.y + to.h)
    }
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

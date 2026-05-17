// ─── Family 2: @startditaa ─────────────────────────────────────────────────────
//
// Real ASCII art rasterizer: 5-pass approach.

use super::shared::{escape_xml, strip_block, svg_header, svg_white_bg};
use crate::diagnostic::Diagnostic;

/// Color hints inside ditaa boxes
fn hint_to_fill(hint: &str) -> Option<&'static str> {
    match hint {
        "cBLU" | "cBlu" => Some("#aad4f5"),
        "cRED" | "cRed" => Some("#f5aaaa"),
        "cGRE" | "cGre" => Some("#aaf5aa"),
        "cYEL" | "cYel" => Some("#f5f5aa"),
        "cBLK" | "cBlk" => Some("#222222"),
        "cWHI" | "cWhi" => Some("#ffffff"),
        "cPNK" | "cPnk" => Some("#f5aad4"),
        "cORA" | "cOra" => Some("#f5d4aa"),
        "cGRA" | "cGra" => Some("#cccccc"),
        "cAAA" | "cAaa" => Some("#dddddd"),
        _ => None,
    }
}

fn ditaa_tag_kind(text: &str) -> Option<ShapeKind> {
    if text.contains("{c}") {
        Some(ShapeKind::Diamond)
    } else if text.contains("{d}") {
        Some(ShapeKind::Document)
    } else if text.contains("{io}") {
        Some(ShapeKind::Io)
    } else if text.contains("{mo}") {
        Some(ShapeKind::ManualOperation)
    } else if text.contains("{o}") {
        Some(ShapeKind::Ellipse)
    } else if text.contains("{s}") {
        Some(ShapeKind::Cylinder)
    } else if text.contains("{tr}") {
        Some(ShapeKind::Trapezoid)
    } else {
        None
    }
}

fn strip_ditaa_tags(text: &str) -> String {
    text.split_whitespace()
        .filter(|part| {
            !matches!(
                *part,
                "{c}" | "{d}" | "{io}" | "{mo}" | "{o}" | "{s}" | "{tr}"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Shape types detected in the grid.
#[derive(Debug, Clone)]
enum ShapeKind {
    Rect,
    RoundedRect,
    Document,
    Cylinder,
    Diamond,
    Io,
    ManualOperation,
    Ellipse,
    Trapezoid,
}

impl ShapeKind {
    fn attr(&self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::RoundedRect => "rounded",
            Self::Document => "document",
            Self::Cylinder => "storage",
            Self::Diamond => "choice",
            Self::Io => "io",
            Self::ManualOperation => "manual-operation",
            Self::Ellipse => "ellipse",
            Self::Trapezoid => "trapezoid",
        }
    }
}

/// A detected shape.
#[derive(Debug, Clone)]
struct Shape {
    kind: ShapeKind,
    r1: usize,
    c1: usize,
    r2: usize,
    c2: usize,
    fill: String,
    dashed: bool,
    text_lines: Vec<(usize, String)>, // (row_idx, text)
}

/// A connector arrow or line.
#[derive(Debug, Clone)]
struct Connector {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    has_head_end: bool,
    has_head_start: bool,
    dashed: bool,
}

pub(super) fn render_ditaa(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startditaa", "@endditaa");
    let options = parse_ditaa_options(source.lines().next().unwrap_or(""));

    if body.trim().is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa body is empty",
        ));
    }

    // Build padded grid
    let lines: Vec<Vec<char>> = body.lines().map(|l| l.chars().collect()).collect();
    if lines.is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa has no grid content",
        ));
    }

    let cell_w = 10i32 * options.scale;
    let cell_h = 16i32 * options.scale;
    let grid_rows = lines.len();
    let grid_cols = lines.iter().map(|r| r.len()).max().unwrap_or(0);
    let title_h = if title.is_some() { 28i32 } else { 0 };
    let margin = 16i32;
    let svg_w = grid_cols as i32 * cell_w + margin * 2;
    let svg_h = grid_rows as i32 * cell_h + margin * 2 + title_h;

    let get = |r: usize, c: usize| -> char {
        lines
            .get(r)
            .and_then(|row| row.get(c))
            .copied()
            .unwrap_or(' ')
    };

    // ── Pass 1: detect shapes ──────────────────────────────────────────────────

    let mut shapes: Vec<Shape> = Vec::new();
    // Track which cells are claimed by a shape
    let mut claimed = vec![vec![false; grid_cols + 1]; grid_rows + 1];

    for r1 in 0..grid_rows {
        for c1 in 0..grid_cols {
            let tl = get(r1, c1);
            if tl != '+' && tl != '(' {
                continue;
            }
            // Check if this corner has already been claimed as part of a larger shape
            if claimed[r1][c1] {
                continue;
            }
            let rounded_start = tl == '(';

            // Find right corner on same row
            let row_len = lines[r1].len();
            let mut c2_candidates: Vec<usize> = Vec::new();
            let mut cc = c1 + 1;
            while cc < row_len {
                let ch = get(r1, cc);
                if ch == '+' || ch == ')' {
                    // Verify top edge is continuous
                    let top_ok = (c1 + 1..cc).all(|c| matches!(get(r1, c), '-' | '=' | ' '));
                    if top_ok {
                        c2_candidates.push(cc);
                    }
                    break; // only try nearest right corner
                } else if !matches!(ch, '-' | '=' | ' ') {
                    break;
                }
                cc += 1;
            }

            for c2 in c2_candidates {
                let tr = get(r1, c2);
                let rounded_end = tr == ')';

                // Find bottom corners
                let mut r2_candidates: Vec<usize> = Vec::new();
                let mut rr = r1 + 1;
                while rr < grid_rows {
                    let bl = get(rr, c1);
                    let br = get(rr, c2);
                    if (bl == '+' || bl == '(') && (br == '+' || br == ')') {
                        // Verify all edges
                        let bot_ok = (c1 + 1..c2).all(|c| matches!(get(rr, c), '-' | '=' | ' '));
                        let left_ok =
                            (r1 + 1..rr).all(|r| matches!(get(r, c1), '|' | ':' | '+' | ' '));
                        let right_ok =
                            (r1 + 1..rr).all(|r| matches!(get(r, c2), '|' | ':' | '+' | ' '));
                        if bot_ok && left_ok && right_ok {
                            r2_candidates.push(rr);
                        }
                        break;
                    } else if !matches!(bl, '|' | ':' | ' ' | '+') {
                        break;
                    }
                    rr += 1;
                }

                for r2 in r2_candidates {
                    // Determine fill by scanning for color hints inside box
                    let mut fill = "#f0f4ff".to_string();
                    let mut dashed = false;
                    let mut tag_kind: Option<ShapeKind> = None;
                    let mut text_lines: Vec<(usize, String)> = Vec::new();

                    for row_idx in (r1 + 1)..r2 {
                        let mut inner = String::new();
                        for ci in (c1 + 1)..c2 {
                            let ch = get(row_idx, ci);
                            if !matches!(ch, '|' | ':') {
                                inner.push(ch);
                            }
                        }
                        let trimmed_inner = inner.trim().to_string();

                        // Color hint detection
                        for word in trimmed_inner.split_whitespace() {
                            if let Some(f) = hint_to_fill(word) {
                                fill = f.to_string();
                            }
                        }
                        tag_kind = tag_kind.or_else(|| ditaa_tag_kind(&trimmed_inner));

                        // Check for dashed edges
                        if (c1 + 1..c2).any(|c| get(r1, c) == '=')
                            || (r1 + 1..r2).any(|r| get(r, c1) == ':')
                        {
                            dashed = true;
                        }

                        // Remove color hints from display text
                        let display: String = trimmed_inner
                            .split_whitespace()
                            .filter(|w| hint_to_fill(w).is_none())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let display = strip_ditaa_tags(&display);

                        if !display.is_empty() {
                            text_lines.push((row_idx, display));
                        }
                    }

                    // Determine shape kind
                    let kind = if let Some(kind) = tag_kind {
                        kind
                    } else if rounded_start || rounded_end {
                        ShapeKind::RoundedRect
                    } else {
                        // Check for cylinder: top row has '(' at c1+1 and ')' at c2-1
                        let maybe_cyl = c2 > c1 + 2
                            && (r1 + 1..r2).all(|r| get(r, c1) == '|' && get(r, c2) == '|')
                            && get(r1, c1 + 1) == '('
                            && get(r1, c2 - 1) == ')';
                        // Check for diamond: /...\ top and \.../ bottom
                        let maybe_diamond = c2 > c1 + 2
                            && get(r1, c1 + 1) == '/'
                            && get(r1, c2 - 1) == '\\'
                            && get(r2, c1 + 1) == '\\'
                            && get(r2, c2 - 1) == '/';
                        // Check for document: bottom row has '~' wave
                        let maybe_doc = (c1 + 1..c2).any(|c| get(r2, c) == '~');

                        if maybe_diamond {
                            ShapeKind::Diamond
                        } else if maybe_cyl {
                            ShapeKind::Cylinder
                        } else if maybe_doc {
                            ShapeKind::Document
                        } else {
                            ShapeKind::Rect
                        }
                    };

                    // Mark cells as claimed
                    for row in claimed.iter_mut().take(r2 + 1).skip(r1) {
                        for c in c1..=c2 {
                            if c < row.len() {
                                row[c] = true;
                            }
                        }
                    }

                    shapes.push(Shape {
                        kind,
                        r1,
                        c1,
                        r2,
                        c2,
                        fill,
                        dashed,
                        text_lines,
                    });
                }
            }
        }
    }

    // ── Pass 2: connector detection ────────────────────────────────────────────

    let mut connectors: Vec<Connector> = Vec::new();

    // Horizontal connectors (sequences of '-' or '=' not part of shape border)
    for (row_idx, row) in lines.iter().enumerate() {
        let mut c = 0usize;
        while c < row.len() {
            let ch = row[c];
            if ch == '<' && c + 1 < row.len() && row[c + 1] == '-' {
                // Left-pointing arrow start
                let c_start = c;
                c += 1;
                let dashed = row[c] == '=';
                while c < row.len() && matches!(row[c], '-' | '=' | '+') {
                    c += 1;
                }
                let c_end = c;
                // Check not on shape border
                let is_border = shapes.iter().any(|s| {
                    (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2
                });
                if !is_border {
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + c_end as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    connectors.push(Connector {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                        has_head_end: false,
                        has_head_start: true,
                        dashed,
                    });
                }
            } else if matches!(ch, '-' | '=') {
                let c_start = c;
                let dashed = ch == '=';
                while c < row.len() && matches!(row[c], '-' | '=' | '+') {
                    c += 1;
                }
                let has_head = c < row.len() && row[c] == '>';
                if has_head {
                    c += 1;
                }
                let c_end = c;

                let is_border = shapes.iter().any(|s| {
                    (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2
                });
                if !is_border && c_end > c_start {
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + (c_end - if has_head { 1 } else { 0 }) as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    connectors.push(Connector {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                        has_head_end: has_head,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else {
                c += 1;
            }
        }
    }

    // Vertical connectors (sequences of '|' or ':')
    for col_idx in 0..grid_cols {
        let mut r = 0usize;
        while r < grid_rows {
            let ch = get(r, col_idx);
            if ch == '^' && r + 1 < grid_rows && matches!(get(r + 1, col_idx), '|' | ':') {
                // Upward arrow
                let r_start = r;
                r += 1;
                let dashed = get(r, col_idx) == ':';
                while r < grid_rows && matches!(get(r, col_idx), '|' | ':' | '+') {
                    r += 1;
                }
                let r_end = r;
                let is_border = shapes.iter().any(|s| {
                    (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2
                });
                if !is_border {
                    let x = margin + col_idx as i32 * cell_w + cell_w / 2;
                    let y1 = margin + title_h + r_end as i32 * cell_h;
                    let y2 = margin + title_h + r_start as i32 * cell_h;
                    connectors.push(Connector {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                        has_head_end: true,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else if matches!(ch, '|' | ':') {
                let r_start = r;
                let dashed = ch == ':';
                while r < grid_rows && matches!(get(r, col_idx), '|' | ':' | '+') {
                    r += 1;
                }
                // Check if next char is 'v'
                let has_head = r < grid_rows && get(r, col_idx) == 'v';
                if has_head {
                    r += 1;
                }
                let r_end = r;
                let is_border = shapes.iter().any(|s| {
                    (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2
                });
                if !is_border && r_end > r_start {
                    let x = margin + col_idx as i32 * cell_w + cell_w / 2;
                    let y1 = margin + title_h + r_start as i32 * cell_h;
                    let y2 = margin + title_h + r_end as i32 * cell_h;
                    connectors.push(Connector {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                        has_head_end: has_head,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else {
                r += 1;
            }
        }
    }

    // Diagonal connectors, a common ditaa idiom for loose ASCII wiring.
    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            if !matches!(ch, '/' | '\\') {
                continue;
            }
            let in_shape = shapes
                .iter()
                .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2);
            if in_shape {
                continue;
            }
            let x = margin + c as i32 * cell_w + cell_w / 2;
            let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
            let (x1, y1, x2, y2) = if ch == '/' {
                (
                    x - cell_w / 2,
                    y + cell_h / 2,
                    x + cell_w / 2,
                    y - cell_h / 2,
                )
            } else {
                (
                    x - cell_w / 2,
                    y - cell_h / 2,
                    x + cell_w / 2,
                    y + cell_h / 2,
                )
            };
            let has_head_start = match ch {
                '/' => {
                    let r = (row_idx + 1).min(grid_rows.saturating_sub(1));
                    (c.saturating_sub(3)..=c + 1).any(|cc| get(r, cc) == '<')
                        || (c.saturating_sub(3)..=c).any(|cc| get(row_idx, cc) == '<')
                        || (0..grid_cols).any(|cc| get(r, cc) == '<')
                }
                '\\' => {
                    let r = row_idx.saturating_sub(1);
                    (c.saturating_sub(3)..=c + 1).any(|cc| get(r, cc) == '<')
                        || (c.saturating_sub(3)..=c).any(|cc| get(row_idx, cc) == '<')
                        || (0..grid_cols).any(|cc| get(r, cc) == '<')
                }
                _ => false,
            };
            let has_head_end = match ch {
                '/' => {
                    c + 1 < grid_cols && row_idx > 0 && matches!(get(row_idx - 1, c + 1), '>' | '^')
                }
                '\\' => {
                    c + 1 < grid_cols
                        && row_idx + 1 < grid_rows
                        && matches!(get(row_idx + 1, c + 1), '>' | 'v')
                }
                _ => false,
            };
            connectors.push(Connector {
                x1,
                y1,
                x2,
                y2,
                has_head_end,
                has_head_start,
                dashed: false,
            });
        }
    }

    // ── Pass 3: SVG emission ──────────────────────────────────────────────────

    let mut out = String::new();
    out.push_str(&svg_header(svg_w, svg_h));
    if !options.transparent {
        if let Some(background) = &options.background {
            out.push_str(&format!(
                "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
                escape_xml(background)
            ));
        } else {
            out.push_str(svg_white_bg());
        }
    }

    // Arrow markers
    out.push_str(
        "<defs>\
         <marker id=\"da\" markerWidth=\"8\" markerHeight=\"6\" refX=\"6\" refY=\"3\" orient=\"auto\">\
         <path d=\"M0,0 L0,6 L8,3 z\" fill=\"#444\"/></marker>\
         <marker id=\"dah\" markerWidth=\"8\" markerHeight=\"6\" refX=\"2\" refY=\"3\" orient=\"auto\">\
         <path d=\"M8,0 L8,6 L0,3 z\" fill=\"#444\"/></marker>\
         </defs>",
    );
    if options.shadow {
        out.push_str("<defs><filter id=\"ditaa-shadow\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"140%\"><feDropShadow dx=\"2\" dy=\"2\" stdDeviation=\"1.5\" flood-color=\"#00000033\"/></filter></defs>");
    }

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            svg_w / 2, margin + 16, escape_xml(t)
        ));
    }

    // Draw shapes
    for shape in &shapes {
        let rx = margin + shape.c1 as i32 * cell_w;
        let ry = margin + title_h + shape.r1 as i32 * cell_h;
        let rw = (shape.c2 - shape.c1) as i32 * cell_w;
        let rh = (shape.r2 - shape.r1) as i32 * cell_h;
        let stroke = if shape.dashed {
            "stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let filter = if options.shadow {
            "filter=\"url(#ditaa-shadow)\""
        } else {
            ""
        };
        let shape_attr = shape.kind.attr();

        match shape.kind {
            ShapeKind::Rect => {
                out.push_str(&format!(
                    "<rect data-ditaa-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    shape_attr, rx, ry, rw, rh, shape.fill, stroke, filter
                ));
            }
            ShapeKind::RoundedRect => {
                out.push_str(&format!(
                    "<rect data-ditaa-shape=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" ry=\"12\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    shape_attr, rx, ry, rw, rh, shape.fill, stroke, filter
                ));
            }
            ShapeKind::Document => {
                // Draw as rect with curved bottom
                let cx = rx + rw / 2;
                let bot_y = ry + rh;
                out.push_str(&format!(
                    "<path data-ditaa-shape=\"{}\" d=\"M {},{} L {},{} L {},{} Q {},{} {},{} Q {},{} {},{}  Z\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    shape_attr,
                    rx, ry,
                    rx + rw, ry,
                    rx + rw, bot_y - 8,
                    cx + rw / 4, bot_y + 6, cx, bot_y - 4,
                    cx - rw / 4, bot_y - 14, rx, bot_y - 8,
                    shape.fill, stroke
                ));
            }
            ShapeKind::Cylinder => {
                let cx = rx + rw / 2;
                let ell_ry = 6i32;
                out.push_str(&format!(
                    "<g data-ditaa-shape=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>\
                     <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\"/>\
                     <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"#3344aa\" stroke-width=\"1\"/></g>",
                    shape_attr, rx, ry + ell_ry, rw, rh - ell_ry, shape.fill, stroke,
                    cx, ry + ell_ry, rw / 2, ell_ry, shape.fill,
                    cx, ry + rh, rw / 2, ell_ry
                ));
            }
            ShapeKind::Diamond => {
                let cx = rx + rw / 2;
                let cy = ry + rh / 2;
                out.push_str(&format!(
                    "<polygon data-ditaa-shape=\"{}\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    shape_attr,
                    cx, ry,
                    rx + rw, cy,
                    cx, ry + rh,
                    rx, cy,
                    shape.fill, stroke
                ));
            }
            ShapeKind::Io => {
                let slant = (rw / 6).max(8);
                out.push_str(&format!(
                    "<polygon data-ditaa-shape=\"{}\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    shape_attr,
                    rx + slant, ry,
                    rx + rw, ry,
                    rx + rw - slant, ry + rh,
                    rx, ry + rh,
                    shape.fill,
                    stroke,
                    filter
                ));
            }
            ShapeKind::ManualOperation | ShapeKind::Trapezoid => {
                let slant = (rw / 6).max(8);
                out.push_str(&format!(
                    "<polygon data-ditaa-shape=\"{}\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    shape_attr,
                    rx, ry,
                    rx + rw, ry,
                    rx + rw - slant, ry + rh,
                    rx + slant, ry + rh,
                    shape.fill,
                    stroke,
                    filter
                ));
            }
            ShapeKind::Ellipse => {
                out.push_str(&format!(
                    "<ellipse data-ditaa-shape=\"{}\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    shape_attr,
                    rx + rw / 2,
                    ry + rh / 2,
                    (rw / 2).max(8),
                    (rh / 2).max(8),
                    shape.fill,
                    stroke,
                    filter
                ));
            }
        }

        // Render text inside shape
        for (row_idx, text) in &shape.text_lines {
            let tx = rx + rw / 2;
            let ty = margin + title_h + *row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#111\">{}</text>",
                tx, ty, escape_xml(text)
            ));
        }
    }

    // Draw connectors
    for conn in &connectors {
        let dash = if conn.dashed {
            " stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let mut marker_end = "";
        let mut marker_start = "";
        if conn.has_head_end {
            marker_end = " marker-end=\"url(#da)\"";
        }
        if conn.has_head_start {
            marker_start = " marker-start=\"url(#dah)\"";
        }
        out.push_str(&format!(
            "<line class=\"ditaa-connector\" data-ditaa-arrow-start=\"{}\" data-ditaa-arrow-end=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\"{}{}{}/>",
            conn.has_head_start, conn.has_head_end, conn.x1, conn.y1, conn.x2, conn.y2, dash, marker_end, marker_start
        ));
    }

    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            if ch != '+' {
                continue;
            }
            let in_shape = shapes
                .iter()
                .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2);
            if in_shape {
                continue;
            }
            let horizontal = matches!(get(row_idx, c.saturating_sub(1)), '-' | '=')
                || matches!(get(row_idx, c + 1), '-' | '=' | '>');
            let vertical = matches!(get(row_idx.saturating_sub(1), c), '|' | ':' | '^')
                || matches!(get(row_idx + 1, c), '|' | ':' | 'v');
            if horizontal && vertical {
                let x = margin + c as i32 * cell_w + cell_w / 2;
                let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                out.push_str(&format!(
                    "<circle class=\"ditaa-junction\" data-ditaa-junction=\"true\" cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#444\"/>",
                    x, y
                ));
            }
        }
    }

    // ── Pass 4: render unclaimed text ─────────────────────────────────────────

    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            // Skip if inside a shape region
            let in_shape = shapes
                .iter()
                .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2);
            if in_shape {
                continue;
            }
            // Skip structural chars and arrows
            if matches!(
                ch,
                '+' | '-'
                    | '|'
                    | '='
                    | ':'
                    | '>'
                    | '<'
                    | 'v'
                    | '^'
                    | ' '
                    | '~'
                    | '('
                    | ')'
                    | '/'
                    | '\\'
            ) {
                continue;
            }
            let tx = margin + c as i32 * cell_w;
            let ty = margin + title_h + row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#222\">{}</text>",
                tx, ty, escape_xml(&ch.to_string())
            ));
        }
    }

    out.push_str("</svg>");
    Ok(out)
}

#[derive(Debug, Clone)]
struct DitaaOptions {
    scale: i32,
    transparent: bool,
    shadow: bool,
    background: Option<String>,
}

fn parse_ditaa_options(first_line: &str) -> DitaaOptions {
    let mut options = DitaaOptions {
        scale: 1,
        transparent: false,
        shadow: false,
        background: None,
    };
    let lower = first_line.to_ascii_lowercase();
    if let Some(pos) = lower.find("scale=") {
        let n: String = lower[pos + 6..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(v) = n.parse::<i32>() {
            options.scale = v.clamp(1, 4);
        }
    }
    if lower.contains("transparent=true") || lower.contains("transparent=yes") {
        options.transparent = true;
    }
    if lower.contains("shadow=true") || lower.contains("shadow=yes") {
        options.shadow = true;
    }
    if let Some(pos) = lower.find("background=") {
        let value: String = first_line[pos + "background=".len()..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if !value.is_empty() {
            options.background = Some(value);
        }
    }
    options
}

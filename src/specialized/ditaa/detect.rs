use super::model::{ditaa_tag_kind, hint_to_fill, strip_ditaa_tags, DitaaGrid, Shape, ShapeKind};

pub(super) fn detect_shapes(grid: &DitaaGrid) -> Vec<Shape> {
    let mut shapes: Vec<Shape> = Vec::new();
    let mut claimed = vec![vec![false; grid.cols + 1]; grid.rows + 1];

    for r1 in 0..grid.rows {
        for c1 in 0..grid.cols {
            let tl = grid.get(r1, c1);
            if tl != '+' && tl != '(' {
                continue;
            }
            if claimed[r1][c1] {
                continue;
            }
            let rounded_start = tl == '(';

            let row_len = grid.lines[r1].len();
            let mut c2_candidates: Vec<usize> = Vec::new();
            let mut cc = c1 + 1;
            while cc < row_len {
                let ch = grid.get(r1, cc);
                if ch == '+' || ch == ')' {
                    let top_ok = (c1 + 1..cc).all(|c| matches!(grid.get(r1, c), '-' | '=' | ' '));
                    if top_ok {
                        c2_candidates.push(cc);
                    }
                    break;
                } else if !matches!(ch, '-' | '=' | ' ') {
                    break;
                }
                cc += 1;
            }

            for c2 in c2_candidates {
                let tr = grid.get(r1, c2);
                let rounded_end = tr == ')';

                let mut r2_candidates: Vec<usize> = Vec::new();
                let mut rr = r1 + 1;
                while rr < grid.rows {
                    let bl = grid.get(rr, c1);
                    let br = grid.get(rr, c2);
                    if (bl == '+' || bl == '(') && (br == '+' || br == ')') {
                        let bot_ok =
                            (c1 + 1..c2).all(|c| matches!(grid.get(rr, c), '-' | '=' | ' '));
                        let left_ok =
                            (r1 + 1..rr).all(|r| matches!(grid.get(r, c1), '|' | ':' | '+' | ' '));
                        let right_ok =
                            (r1 + 1..rr).all(|r| matches!(grid.get(r, c2), '|' | ':' | '+' | ' '));
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
                    let mut fill = "#f0f4ff".to_string();
                    let mut dashed = false;
                    let mut tag_kind: Option<ShapeKind> = None;
                    let mut text_lines: Vec<(usize, String)> = Vec::new();

                    for row_idx in (r1 + 1)..r2 {
                        let mut inner = String::new();
                        for ci in (c1 + 1)..c2 {
                            let ch = grid.get(row_idx, ci);
                            if !matches!(ch, '|' | ':') {
                                inner.push(ch);
                            }
                        }
                        let trimmed_inner = inner.trim().to_string();

                        for word in trimmed_inner.split_whitespace() {
                            if let Some(f) = hint_to_fill(word) {
                                fill = f.to_string();
                            }
                        }
                        tag_kind = tag_kind.or_else(|| ditaa_tag_kind(&trimmed_inner));

                        if (c1 + 1..c2).any(|c| grid.get(r1, c) == '=')
                            || (r1 + 1..r2).any(|r| grid.get(r, c1) == ':')
                        {
                            dashed = true;
                        }

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

                    let kind = tag_kind.unwrap_or_else(|| {
                        if rounded_start || rounded_end {
                            ShapeKind::RoundedRect
                        } else {
                            infer_shape_kind(grid, r1, c1, r2, c2)
                        }
                    });

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

    shapes
}

fn infer_shape_kind(grid: &DitaaGrid, r1: usize, c1: usize, r2: usize, c2: usize) -> ShapeKind {
    let maybe_cyl = c2 > c1 + 2
        && (r1 + 1..r2).all(|r| grid.get(r, c1) == '|' && grid.get(r, c2) == '|')
        && grid.get(r1, c1 + 1) == '('
        && grid.get(r1, c2 - 1) == ')';
    let maybe_diamond = c2 > c1 + 2
        && grid.get(r1, c1 + 1) == '/'
        && grid.get(r1, c2 - 1) == '\\'
        && grid.get(r2, c1 + 1) == '\\'
        && grid.get(r2, c2 - 1) == '/';
    let maybe_doc = (c1 + 1..c2).any(|c| grid.get(r2, c) == '~');

    if maybe_diamond {
        ShapeKind::Diamond
    } else if maybe_cyl {
        ShapeKind::Cylinder
    } else if maybe_doc {
        ShapeKind::Document
    } else {
        ShapeKind::Rect
    }
}

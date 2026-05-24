use super::model::{Connector, DitaaGrid, Shape};

pub(super) fn detect_connectors(grid: &DitaaGrid, shapes: &[Shape]) -> Vec<Connector> {
    let mut connectors = Vec::new();
    detect_horizontal_connectors(grid, shapes, &mut connectors);
    detect_vertical_connectors(grid, shapes, &mut connectors);
    detect_diagonal_connectors(grid, shapes, &mut connectors);
    connectors
}

fn detect_horizontal_connectors(
    grid: &DitaaGrid,
    shapes: &[Shape],
    connectors: &mut Vec<Connector>,
) {
    for (row_idx, row) in grid.lines.iter().enumerate() {
        let mut c = 0usize;
        while c < row.len() {
            let ch = row[c];
            if ch == '<' && c + 1 < row.len() && row[c + 1] == '-' {
                let c_start = c;
                c += 1;
                let dashed = row[c] == '=';
                while c < row.len() && matches!(row[c], '-' | '=' | '+') {
                    c += 1;
                }
                let c_end = c;
                if !is_horizontal_shape_border(shapes, row_idx, c_start, c_end) {
                    let y = grid.y_for_row(row_idx) + grid.cell_h / 2;
                    connectors.push(Connector {
                        x1: grid.x_for_col(c_start),
                        y1: y,
                        x2: grid.x_for_col(c_end),
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

                if !is_horizontal_shape_border(shapes, row_idx, c_start, c_end) && c_end > c_start {
                    let y = grid.y_for_row(row_idx) + grid.cell_h / 2;
                    connectors.push(Connector {
                        x1: grid.x_for_col(c_start),
                        y1: y,
                        x2: grid.x_for_col(c_end - if has_head { 1 } else { 0 }),
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
}

fn detect_vertical_connectors(grid: &DitaaGrid, shapes: &[Shape], connectors: &mut Vec<Connector>) {
    for col_idx in 0..grid.cols {
        let mut r = 0usize;
        while r < grid.rows {
            let ch = grid.get(r, col_idx);
            if ch == '^' && r + 1 < grid.rows && matches!(grid.get(r + 1, col_idx), '|' | ':') {
                let r_start = r;
                r += 1;
                let dashed = grid.get(r, col_idx) == ':';
                while r < grid.rows && matches!(grid.get(r, col_idx), '|' | ':' | '+') {
                    r += 1;
                }
                let r_end = r;
                if !is_vertical_shape_border(shapes, col_idx, r_start, r_end) {
                    let x = grid.x_for_col(col_idx) + grid.cell_w / 2;
                    connectors.push(Connector {
                        x1: x,
                        y1: grid.y_for_row(r_end),
                        x2: x,
                        y2: grid.y_for_row(r_start),
                        has_head_end: true,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else if matches!(ch, '|' | ':') {
                let r_start = r;
                let dashed = ch == ':';
                while r < grid.rows && matches!(grid.get(r, col_idx), '|' | ':' | '+') {
                    r += 1;
                }
                let has_head = r < grid.rows && grid.get(r, col_idx) == 'v';
                if has_head {
                    r += 1;
                }
                let r_end = r;
                if !is_vertical_shape_border(shapes, col_idx, r_start, r_end) && r_end > r_start {
                    let x = grid.x_for_col(col_idx) + grid.cell_w / 2;
                    connectors.push(Connector {
                        x1: x,
                        y1: grid.y_for_row(r_start),
                        x2: x,
                        y2: grid.y_for_row(r_end),
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
}

fn detect_diagonal_connectors(grid: &DitaaGrid, shapes: &[Shape], connectors: &mut Vec<Connector>) {
    for (row_idx, row) in grid.lines.iter().enumerate() {
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
            let x = grid.x_for_col(c) + grid.cell_w / 2;
            let y = grid.y_for_row(row_idx) + grid.cell_h / 2;
            let (x1, y1, x2, y2) = if ch == '/' {
                (
                    x - grid.cell_w / 2,
                    y + grid.cell_h / 2,
                    x + grid.cell_w / 2,
                    y - grid.cell_h / 2,
                )
            } else {
                (
                    x - grid.cell_w / 2,
                    y - grid.cell_h / 2,
                    x + grid.cell_w / 2,
                    y + grid.cell_h / 2,
                )
            };
            connectors.push(Connector {
                x1,
                y1,
                x2,
                y2,
                has_head_end: diagonal_head_end(grid, row_idx, c, ch),
                has_head_start: diagonal_head_start(grid, row_idx, c, ch),
                dashed: false,
            });
        }
    }
}

fn is_horizontal_shape_border(
    shapes: &[Shape],
    row_idx: usize,
    c_start: usize,
    c_end: usize,
) -> bool {
    shapes
        .iter()
        .any(|s| (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2)
}

fn is_vertical_shape_border(
    shapes: &[Shape],
    col_idx: usize,
    r_start: usize,
    r_end: usize,
) -> bool {
    shapes
        .iter()
        .any(|s| (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2)
}

fn diagonal_head_start(grid: &DitaaGrid, row_idx: usize, c: usize, ch: char) -> bool {
    match ch {
        '/' => {
            let r = (row_idx + 1).min(grid.rows.saturating_sub(1));
            (c.saturating_sub(3)..=c + 1).any(|cc| grid.get(r, cc) == '<')
                || (c.saturating_sub(3)..=c).any(|cc| grid.get(row_idx, cc) == '<')
                || (0..grid.cols).any(|cc| grid.get(r, cc) == '<')
        }
        '\\' => {
            let r = row_idx.saturating_sub(1);
            (c.saturating_sub(3)..=c + 1).any(|cc| grid.get(r, cc) == '<')
                || (c.saturating_sub(3)..=c).any(|cc| grid.get(row_idx, cc) == '<')
                || (0..grid.cols).any(|cc| grid.get(r, cc) == '<')
        }
        _ => false,
    }
}

fn diagonal_head_end(grid: &DitaaGrid, row_idx: usize, c: usize, ch: char) -> bool {
    match ch {
        '/' => {
            c + 1 < grid.cols && row_idx > 0 && matches!(grid.get(row_idx - 1, c + 1), '>' | '^')
        }
        '\\' => {
            c + 1 < grid.cols
                && row_idx + 1 < grid.rows
                && matches!(grid.get(row_idx + 1, c + 1), '>' | 'v')
        }
        _ => false,
    }
}

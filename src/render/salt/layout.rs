use super::model::SaltCellRender;

pub(super) struct SaltRenderedCell<'a> {
    pub(super) cell: &'a SaltCellRender,
    pub(super) width: i32,
    pub(super) colspan: usize,
}

pub(super) fn salt_row_layout<'a>(
    cells: &'a [SaltCellRender],
    col_widths: &[i32],
    min_cell_w: i32,
) -> Vec<SaltRenderedCell<'a>> {
    let mut rendered: Vec<SaltRenderedCell<'a>> = Vec::new();
    for (col_idx, cell) in cells.iter().enumerate() {
        let width = col_widths.get(col_idx).copied().unwrap_or(min_cell_w);
        if matches!(cell, SaltCellRender::TableSpan) {
            if let Some(previous) = rendered.last_mut() {
                previous.width += width;
                previous.colspan += 1;
            } else {
                rendered.push(SaltRenderedCell {
                    cell,
                    width,
                    colspan: 1,
                });
            }
        } else {
            rendered.push(SaltRenderedCell {
                cell,
                width,
                colspan: 1,
            });
        }
    }
    rendered
}

pub(super) fn is_salt_separator_row(cells: &[SaltCellRender]) -> bool {
    let mut saw_separator = false;
    for cell in cells {
        match cell {
            SaltCellRender::Label(text) => {
                let t = text.trim();
                if t.is_empty() {
                    continue;
                }
                // `---`/`--` dash runs, `..` dotted separator, `==` thick separator.
                if t.chars().all(|c| c == '-')
                    || t.chars().all(|c| c == '.')
                    || t.chars().all(|c| c == '=')
                {
                    saw_separator = true;
                    continue;
                }
                return false;
            }
            _ => return false,
        }
    }
    saw_separator
}

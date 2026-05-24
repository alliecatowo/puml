#[derive(Debug, Clone)]
pub(super) enum ShapeKind {
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
    pub(super) fn attr(&self) -> &'static str {
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

#[derive(Debug, Clone)]
pub(super) struct Shape {
    pub(super) kind: ShapeKind,
    pub(super) r1: usize,
    pub(super) c1: usize,
    pub(super) r2: usize,
    pub(super) c2: usize,
    pub(super) fill: String,
    pub(super) dashed: bool,
    pub(super) text_lines: Vec<(usize, String)>,
}

#[derive(Debug, Clone)]
pub(super) struct Connector {
    pub(super) x1: i32,
    pub(super) y1: i32,
    pub(super) x2: i32,
    pub(super) y2: i32,
    pub(super) has_head_end: bool,
    pub(super) has_head_start: bool,
    pub(super) dashed: bool,
}

pub(super) struct DitaaGrid {
    pub(super) lines: Vec<Vec<char>>,
    pub(super) rows: usize,
    pub(super) cols: usize,
    pub(super) cell_w: i32,
    pub(super) cell_h: i32,
    pub(super) margin: i32,
    pub(super) title_h: i32,
    pub(super) svg_w: i32,
    pub(super) svg_h: i32,
}

impl DitaaGrid {
    pub(super) fn new(body: &str, scale: i32, has_title: bool) -> Self {
        let lines: Vec<Vec<char>> = body.lines().map(|line| line.chars().collect()).collect();
        let rows = lines.len();
        let cols = lines.iter().map(|row| row.len()).max().unwrap_or(0);
        let cell_w = 10i32 * scale;
        let cell_h = 16i32 * scale;
        let margin = 16i32;
        let title_h = if has_title { 28i32 } else { 0 };
        let svg_w = cols as i32 * cell_w + margin * 2;
        let svg_h = rows as i32 * cell_h + margin * 2 + title_h;

        Self {
            lines,
            rows,
            cols,
            cell_w,
            cell_h,
            margin,
            title_h,
            svg_w,
            svg_h,
        }
    }

    pub(super) fn get(&self, row: usize, col: usize) -> char {
        self.lines
            .get(row)
            .and_then(|line| line.get(col))
            .copied()
            .unwrap_or(' ')
    }

    pub(super) fn x_for_col(&self, col: usize) -> i32 {
        self.margin + col as i32 * self.cell_w
    }

    pub(super) fn y_for_row(&self, row: usize) -> i32 {
        self.margin + self.title_h + row as i32 * self.cell_h
    }
}

#[derive(Debug, Clone)]
pub(super) struct DitaaOptions {
    pub(super) scale: i32,
    pub(super) transparent: bool,
    pub(super) shadow: bool,
    pub(super) background: Option<String>,
}

impl Default for DitaaOptions {
    fn default() -> Self {
        Self {
            scale: 1,
            transparent: false,
            shadow: false,
            background: None,
        }
    }
}

impl DitaaOptions {
    pub(super) fn parse(first_line: &str) -> Self {
        let mut options = Self::default();
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
}

pub(super) fn hint_to_fill(hint: &str) -> Option<&'static str> {
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

pub(super) fn ditaa_tag_kind(text: &str) -> Option<ShapeKind> {
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

pub(super) fn strip_ditaa_tags(text: &str) -> String {
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

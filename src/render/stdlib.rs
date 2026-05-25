use crate::model::StdlibDocument;
use crate::stdlib::StdlibPackStatus;

use super::{creole_text, escape_text};

pub fn render_stdlib_svg(doc: &StdlibDocument) -> String {
    let width = 920;
    let row_height = 24;
    let pack_rows = doc.packs.len().max(1) as i32;
    let alias_rows = doc.aliases.len().max(1) as i32;
    let entry_preview = doc.entries.iter().take(18).collect::<Vec<_>>();
    let entry_rows = entry_preview.len().max(1) as i32;
    let height = 174 + (pack_rows + alias_rows + entry_rows) * row_height;
    let available_count = doc
        .packs
        .iter()
        .filter(|pack| {
            matches!(
                pack.status,
                StdlibPackStatus::Available | StdlibPackStatus::Builtin
            )
        })
        .count();
    let unavailable_count = doc.packs.len().saturating_sub(available_count);

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" data-stdlib-catalog=\"true\" data-stdlib-entry-count=\"{}\" data-stdlib-pack-count=\"{}\" data-stdlib-unavailable-pack-count=\"{}\">",
        doc.entries.len(),
        available_count,
        unavailable_count
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str("<rect x=\"24\" y=\"24\" width=\"872\" height=\"56\" rx=\"4\" ry=\"4\" fill=\"#f8fafc\" stroke=\"#334155\" stroke-width=\"1.5\"/>");
    out.push_str(&format!(
        "<text x=\"40\" y=\"48\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
        escape_text(doc.title.as_deref().unwrap_or("PlantUML stdlib catalog"))
    ));
    out.push_str(&format!(
        "<text x=\"40\" y=\"68\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{} entries, {} available packs, {} unavailable upstream packs</text>",
        doc.entries.len(),
        available_count,
        unavailable_count
    ));

    let mut y = 112;
    section_header(&mut out, "Packs", y);
    y += 18;
    if doc.packs.is_empty() {
        text_row(&mut out, y, "(none)", "", "#64748b");
        y += row_height;
    } else {
        for pack in &doc.packs {
            let status = match pack.status {
                StdlibPackStatus::Available => "available",
                StdlibPackStatus::Builtin => "builtin",
                StdlibPackStatus::Unavailable => "unavailable",
            };
            let color = match pack.status {
                StdlibPackStatus::Available => "#166534",
                StdlibPackStatus::Builtin => "#0369a1",
                StdlibPackStatus::Unavailable => "#991b1b",
            };
            text_row(
                &mut out,
                y,
                &pack.name,
                &format!(
                    "{status}; files {}; alias paths {}",
                    pack.files, pack.aliases
                ),
                color,
            );
            y += row_height;
        }
    }

    y += 12;
    section_header(&mut out, "Aliases", y);
    y += 18;
    if doc.aliases.is_empty() {
        text_row(&mut out, y, "(none)", "", "#64748b");
        y += row_height;
    } else {
        for (slug, target) in &doc.aliases {
            text_row(&mut out, y, slug, &format!("-> {target}"), "#1d4ed8");
            y += row_height;
        }
    }

    y += 12;
    section_header(&mut out, "Sample Include Paths", y);
    y += 18;
    if entry_preview.is_empty() {
        text_row(&mut out, y, "(none)", "", "#64748b");
    } else {
        for entry in entry_preview {
            let value = if entry.alias {
                format!("-> {}", entry.physical_path)
            } else {
                "direct".to_string()
            };
            text_row(&mut out, y, &entry.path, &value, "#334155");
            y += row_height;
        }
    }

    out.push_str("</svg>");
    out
}

fn section_header(out: &mut String, label: &str, y: i32) {
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
        escape_text(label)
    ));
}

fn text_row(out: &mut String, y: i32, key: &str, value: &str, color: &str) {
    let row_top = y - 16;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{row_top}\" width=\"872\" height=\"24\" fill=\"#ffffff\" stroke=\"#e2e8f0\" stroke-width=\"1\"/>"
    ));
    out.push_str(&creole_text(
        40,
        y,
        "font-family=\"monospace\" font-size=\"12\"",
        key,
        color,
    ));
    out.push_str(&creole_text(
        360,
        y,
        "font-family=\"monospace\" font-size=\"12\"",
        value,
        "#475569",
    ));
}

use crate::ast::{SaltCell, Statement, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::sprites::SpriteRegistry;
use std::collections::BTreeSet;

/// Encode a `SaltGridRow` cell list into the compact string format used by
/// the Salt renderer. Each cell is prefixed with its type tag so the renderer
/// can reconstruct the widget kind from the name field.
pub(super) fn encode_salt_cells(cells: Vec<SaltCell>) -> String {
    use SaltCell as SC;
    let cell_strs: Vec<String> = cells
        .into_iter()
        .map(|c| match c {
            SC::Label(t) => format!("L:{t}"),
            SC::Input(t) => format!("I:{t}"),
            SC::Button(t) => format!("B:{t}"),
            SC::Combo(t) => format!("C:{t}"),
            SC::CheckboxChecked(t) => format!("CX:{t}"),
            SC::CheckboxUnchecked(t) => format!("CU:{t}"),
            SC::RadioOn(t) => format!("RO:{t}"),
            SC::RadioOff(t) => format!("RF:{t}"),
        })
        .collect();
    format!("SALT_ROW\x1f{}", cell_strs.join("\x1e"))
}

/// Pre-scan all statements in a `@startsalt` document to collect the names of
/// salt ASCII-art sprites defined via `<<name\n...\n>>` syntax.
///
/// These sprite definitions arrive as `BenignPassthrough` statements (not as
/// `SpriteDef` AST nodes) because the salt ASCII sprite syntax is not understood
/// by the main parser — it is handled in the render layer.  We need to pre-collect
/// these names so that `salt_scan_unsupported` can distinguish a reference to a
/// locally-defined ASCII sprite from a genuinely missing sprite.
pub(super) fn collect_salt_ascii_sprite_names(statements: &[Statement]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for stmt in statements {
        let raw_line = match &stmt.kind {
            StatementKind::BenignPassthrough(line) => line.as_str(),
            _ => continue,
        };
        let trimmed = raw_line.trim();
        // Salt ASCII sprite definition starts with `<<name` (no closing `>>`
        // on the same line).  The name is the first whitespace-delimited token
        // after `<<`.
        if !trimmed.starts_with("<<") || trimmed.ends_with(">>") {
            continue;
        }
        let inner = &trimmed[2..];
        let name = inner
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(">>")
            .trim();
        if !name.is_empty() {
            names.insert(name.to_string());
        }
    }
    names
}

/// Scan a salt `SaltGridRow` cell list for unsupported constructs and push any
/// `W_SALT_UNSUPPORTED_*` warnings onto `warnings`.
///
/// Called from the `SaltGridRow` handler in `normalize_stub_family` so that
/// users who use these constructs learn explicitly why the output may look
/// different from upstream PlantUML rather than getting silent degradation.
///
/// **Unsupported constructs detected:**
///
/// - `W_SALT_UNSUPPORTED_TABLE_SPAN` — a `*` span cell; PUML renders a
///   placeholder box but does not apply real column-merge geometry
///   (PlantUML spec ch14, table row-span).
/// - `W_SALT_UNSUPPORTED_SPRITE_REF` — a `<<name>>` sprite reference inside a
///   cell when no sprite with that name has been defined (either as a
///   `sprite name { … }` declaration or as a salt ASCII sprite `<<name\n...\n>>`).
///   PUML renders a placeholder icon box with the name only.
pub(super) fn salt_scan_unsupported(
    cells: &[SaltCell],
    span: Span,
    sprites: &SpriteRegistry,
    ascii_sprite_names: &BTreeSet<String>,
    warnings: &mut Vec<Diagnostic>,
) {
    for cell in cells {
        let SaltCell::Label(text) = cell else {
            continue;
        };
        let trimmed = text.trim();

        // `*` table span — rendered as placeholder; span geometry not applied.
        if trimmed == "*" {
            warnings.push(
                Diagnostic::warning(
                    "[W_SALT_UNSUPPORTED_TABLE_SPAN] Salt table row-span (`*`) is not fully \
                     supported: the cell renders as a placeholder box without merging column \
                     geometry. See PlantUML spec ch14 for expected behaviour.",
                )
                .with_span(span),
            );
            continue;
        }

        // `<<name>>` sprite reference — warn when the sprite is not defined.
        if let Some(sprite_name) = trimmed
            .strip_prefix("<<")
            .and_then(|r| r.strip_suffix(">>"))
        {
            let name = sprite_name.trim();
            if !name.is_empty() && !sprites.contains_key(name) && !ascii_sprite_names.contains(name)
            {
                warnings.push(
                    Diagnostic::warning(format!(
                        "[W_SALT_UNSUPPORTED_SPRITE_REF] Salt sprite reference `<<{name}>>` \
                         has no matching sprite definition; a placeholder box will be rendered \
                         instead of the actual icon. Define the sprite with `<<{name}>>` \
                         ... `>>` before using it, or use an OpenIconic reference (`<&icon>`)."
                    ))
                    .with_span(span),
                );
            }
        }
    }
}

/// Holds a deferred `<style>` block param for post-loop application.
pub(super) struct StyleParamRecord {
    pub(super) selector: Option<String>,
    pub(super) property: String,
    pub(super) key: Option<String>,
    pub(super) value: String,
    pub(super) span: Span,
}

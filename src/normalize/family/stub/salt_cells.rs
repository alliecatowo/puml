use crate::ast::SaltCell;
use crate::source::Span;

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

/// Holds a deferred `<style>` block param for post-loop application.
pub(super) struct StyleParamRecord {
    pub(super) selector: Option<String>,
    pub(super) property: String,
    pub(super) key: Option<String>,
    pub(super) value: String,
    pub(super) span: Span,
}

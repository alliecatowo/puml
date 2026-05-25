use crate::ast::{DiagramKind, Document, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::model::StdlibDocument;
use crate::normalize::common::{self, RawSyntaxContext};
use crate::normalize::NormalizeOptions;

pub(super) fn normalize_stdlib_catalog(
    document: Document,
    options: &NormalizeOptions,
) -> Result<StdlibDocument, Diagnostic> {
    let mut title = None;
    let mut saw_catalog = false;
    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::StdlibInventory => saw_catalog = true,
            StatementKind::Title(value) => title = Some(value.clone()),
            StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Caption(_)
            | StatementKind::Legend(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::AllowMixing
            | StatementKind::Footbox(_)
            | StatementKind::Scale(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            | StatementKind::HideUnlinked
            | StatementKind::Mainframe(_) => {}
            kind if kind.raw_syntax().is_some() => {
                return Err(common::raw_syntax_diagnostic(
                    kind.raw_syntax().expect("raw syntax guard"),
                    stmt.span,
                    RawSyntaxContext::Family(DiagramKind::Stdlib),
                ));
            }
            other => {
                return Err(Diagnostic::error_code(
                    "E_STDLIB_UNSUPPORTED",
                    format!("unsupported stdlib catalog statement: {other:?}"),
                )
                .with_span(stmt.span));
            }
        }
    }

    if !saw_catalog {
        return Err(Diagnostic::error_code(
            "E_STDLIB_EMPTY",
            "stdlib catalog diagram requires a `stdlib` statement",
        ));
    }

    let root = crate::stdlib::resolve_local_stdlib_root(options.include_root.as_deref())
        .map_err(|msg| Diagnostic::error_code("E_STDLIB_ROOT", msg))?;
    let entries = crate::stdlib::inventory_from_root(&root)
        .map_err(|msg| Diagnostic::error_code("E_STDLIB_INVENTORY", msg))?;
    let packs = crate::stdlib::stdlib_pack_summaries(&entries);
    let aliases = crate::stdlib::STDLIB_ALIASES
        .iter()
        .map(|alias| (alias.slug.to_string(), alias.target.to_string()))
        .collect::<Vec<_>>();
    let missing_packs = crate::stdlib::sorted_missing_stdlib_packs()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

    Ok(StdlibDocument {
        title,
        root: root.display().to_string(),
        entries,
        packs,
        aliases,
        missing_packs,
        warnings: Vec::new(),
    })
}

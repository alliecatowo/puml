use super::*;

pub(super) fn normalize_math(document: Document) -> Result<MathDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    Ok(MathDocument {
        title,
        body: body.join("\n"),
        warnings: Vec::new(),
    })
}

pub(super) fn normalize_ditaa(document: Document) -> Result<DitaaDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    Ok(DitaaDocument {
        title,
        body: body.join("\n"),
        warnings: Vec::new(),
    })
}

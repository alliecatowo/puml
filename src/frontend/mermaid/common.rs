use super::FrontendResult;
use crate::frontend::SourceMap;

pub(super) fn result_from_adapted(original: &str, adapted: String) -> FrontendResult {
    let source_map = SourceMap::line_map(original, &adapted);
    FrontendResult::new(adapted, source_map)
}

pub(super) fn strip_mermaid_comment(line: &str) -> &str {
    line.split_once("%%").map_or(line, |(prefix, _)| prefix)
}

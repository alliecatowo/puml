#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::{ActivityStepKind, DiagramKind, StatementKind};
    use std::fs;
    use tempfile::tempdir;

    mod activity;
mod family;
mod family_regressions;
mod preprocessor_control;
mod preprocessor_includes;
mod sequence;
mod timeline;
}

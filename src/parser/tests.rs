#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::{ActivityStepKind, DiagramKind, StatementKind};
    use std::fs;
    use tempfile::tempdir;

    mod includes {
        use super::*;
        include!("tests/includes.rs");
    }

    mod preprocessor {
        use super::*;
        include!("tests/preprocessor.rs");
    }

    mod sequence {
        use super::*;
        include!("tests/sequence.rs");
    }

    mod family {
        use super::*;
        include!("tests/family.rs");
    }

    mod activity_timeline {
        use super::*;
        include!("tests/activity_timeline.rs");
    }

    mod misc {
        use super::*;
        include!("tests/misc.rs");
    }
}

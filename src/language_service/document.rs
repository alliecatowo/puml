use crate::{parse_with_pipeline_options, Document, FrontendSelection, ParsePipelineOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotOptions {
    pub frontend: FrontendSelection,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            frontend: FrontendSelection::Auto,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DocumentSnapshot {
    pub text: String,
    pub parsed: Option<Document>,
    pub version: i64,
}

impl DocumentSnapshot {
    pub fn new(text: String, version: i64) -> Self {
        Self::with_options(text, version, SnapshotOptions::default())
    }

    pub fn with_options(text: String, version: i64, options: SnapshotOptions) -> Self {
        let parsed = parse_with_pipeline_options(
            &text,
            &ParsePipelineOptions {
                frontend: options.frontend,
                ..ParsePipelineOptions::default()
            },
        )
        .ok();
        Self {
            text,
            parsed,
            version,
        }
    }
}

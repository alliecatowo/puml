use crate::diagnostic::Diagnostic;
use crate::stdlib::{StdlibEntry, StdlibPackSummary};

#[derive(Debug, Clone)]
pub struct StdlibDocument {
    pub title: Option<String>,
    pub root: String,
    pub entries: Vec<StdlibEntry>,
    pub packs: Vec<StdlibPackSummary>,
    pub aliases: Vec<(String, String)>,
    pub missing_packs: Vec<String>,
    pub warnings: Vec<Diagnostic>,
}

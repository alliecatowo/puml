pub mod commands;
pub mod completion;
pub mod diagnostics;
pub mod document;
pub mod document_features;
pub mod navigation;
pub mod protocol;
pub mod render;
pub mod semantic;

pub type Doc = puml::language_service::DocumentSnapshot;

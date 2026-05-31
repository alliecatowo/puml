use crate::diagnostic::Diagnostic;
use crate::model::FamilyOrientation;
use crate::render::graph_layout::EdgeRouting;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChenNodeKind {
    Entity,
    Relationship,
}

#[derive(Debug, Clone)]
pub struct ChenDocument {
    pub nodes: Vec<ChenNode>,
    pub relations: Vec<ChenRelation>,
    pub inheritances: Vec<ChenInheritance>,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub orientation: FamilyOrientation,
    /// Global edge-routing mode selected by `skinparam linetype <value>`.
    /// Defaults to [`EdgeRouting::Polyline`] (straight lines). When set to
    /// [`EdgeRouting::Ortho`], all edges are routed as right-angle elbows
    /// instead of diagonal lines — the only documented use case for
    /// `skinparam linetype` in PlantUML §20.3 (crow's-feet workaround).
    pub edge_routing: EdgeRouting,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct ChenNode {
    pub kind: ChenNodeKind,
    pub id: String,
    pub label: String,
    pub weak: bool,
    pub identifying: bool,
    pub attributes: Vec<ChenAttribute>,
}

#[derive(Debug, Clone)]
pub struct ChenAttribute {
    pub id: String,
    pub label: String,
    pub data_type: Option<String>,
    pub key: bool,
    pub derived: bool,
    pub multivalued: bool,
    pub children: Vec<ChenAttribute>,
}

#[derive(Debug, Clone)]
pub struct ChenRelation {
    pub from: String,
    pub to: String,
    pub cardinality: String,
    pub total_participation: bool,
}

#[derive(Debug, Clone)]
pub struct ChenInheritance {
    pub parent: String,
    pub connector: String,
    pub discriminator: Option<String>,
    pub children: Vec<String>,
}

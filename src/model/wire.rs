use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Default)]
pub struct WireDocument {
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub components: Vec<WireComponent>,
    pub labels: Vec<WireLabel>,
    pub links: Vec<WireLink>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct WireComponent {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: Option<String>,
    pub ports: Vec<WirePort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WirePortSide {
    Top,
    Right,
    Bottom,
    Left,
}

impl WirePortSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WirePort {
    pub id: String,
    pub label: String,
    pub side: WirePortSide,
    pub order: usize,
}

#[derive(Debug, Clone)]
pub struct WireLabel {
    pub id: String,
    pub text: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct WireLink {
    pub id: String,
    pub from: WireEndpoint,
    pub to: WireEndpoint,
    pub label: Option<String>,
    pub directed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireEndpoint {
    pub component: String,
    pub port: Option<String>,
}

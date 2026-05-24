use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) enum DataFamily {
    Json,
    Yaml,
}

impl DataFamily {
    pub(super) fn projection(self) -> &'static str {
        match self {
            DataFamily::Json => "json",
            DataFamily::Yaml => "yaml",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            DataFamily::Json => "JSON",
            DataFamily::Yaml => "YAML",
        }
    }

    pub(super) fn connector_color(self) -> &'static str {
        match self {
            DataFamily::Json => "#94a3b8",
            DataFamily::Yaml => "#ca8a04",
        }
    }

    pub(super) fn connector_dash(self) -> &'static str {
        match self {
            DataFamily::Json => "",
            DataFamily::Yaml => " stroke-dasharray=\"2 2\"",
        }
    }
}

#[derive(Clone)]
pub(super) struct RenderRow {
    pub(super) depth: usize,
    pub(super) label: String,
    pub(super) key: String,
    pub(super) value: Option<String>,
    pub(super) path: Vec<String>,
}

#[derive(Clone)]
pub(super) struct HighlightSpec {
    pub(super) path: Vec<String>,
    pub(super) class_name: Option<String>,
}

#[derive(Clone)]
pub(super) struct RowStyle {
    pub(super) fill: String,
    pub(super) stroke: String,
    pub(super) font_color: String,
    pub(super) font_style: Option<String>,
    pub(super) font_weight: Option<String>,
}

impl RowStyle {
    pub(super) fn json_node() -> Self {
        Self {
            fill: "#f1f5f9".to_string(),
            stroke: "#94a3b8".to_string(),
            font_color: "#0f172a".to_string(),
            font_style: None,
            font_weight: None,
        }
    }

    pub(super) fn yaml_node() -> Self {
        Self {
            fill: "#fef9c3".to_string(),
            stroke: "#ca8a04".to_string(),
            font_color: "#0f172a".to_string(),
            font_style: None,
            font_weight: None,
        }
    }

    pub(super) fn highlight() -> Self {
        Self {
            fill: "#facc15".to_string(),
            stroke: "#d97706".to_string(),
            font_color: "#111827".to_string(),
            font_style: None,
            font_weight: Some("700".to_string()),
        }
    }

    pub(super) fn merge_patch(&self, patch: Option<&StylePatch>) -> Self {
        let Some(patch) = patch else {
            return self.clone();
        };
        Self {
            fill: patch.fill.clone().unwrap_or_else(|| self.fill.clone()),
            stroke: patch.stroke.clone().unwrap_or_else(|| self.stroke.clone()),
            font_color: patch
                .font_color
                .clone()
                .unwrap_or_else(|| self.font_color.clone()),
            font_style: patch.font_style.clone().or_else(|| self.font_style.clone()),
            font_weight: patch
                .font_weight
                .clone()
                .or_else(|| self.font_weight.clone()),
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct StylePatch {
    pub(super) fill: Option<String>,
    pub(super) stroke: Option<String>,
    pub(super) font_color: Option<String>,
    pub(super) font_style: Option<String>,
    pub(super) font_weight: Option<String>,
}

#[derive(Default)]
pub(super) struct StructuredControls {
    pub(super) payload: String,
    pub(super) highlights: Vec<HighlightSpec>,
    pub(super) default_highlight: StylePatch,
    pub(super) class_styles: BTreeMap<String, StylePatch>,
}

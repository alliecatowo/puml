//! Public render artifact contract shared by API, CLI, LSP, WASM, and backends.

use crate::diagnostic::Diagnostic;
use crate::render_core::{
    validate::GeometryMetric, BackendFormat, GeometryIssue, Rect, RenderScene, SceneAvailability,
};

#[derive(Debug)]
pub struct RenderArtifact {
    pub svg: String,
    pub format: BackendFormat,
    pub dimensions: Option<RenderArtifactDimensions>,
    pub diagnostics: Vec<Diagnostic>,
    pub scene_availability: SceneAvailability,
    /// Compatibility field for callers not yet migrated to `scene_contract`.
    ///
    /// New code should use `typed_scene`, `require_typed_scene`, or
    /// `scene_contract` so absence is interpreted through `scene_availability`
    /// instead of as a silent `None`.
    pub scene: Option<RenderScene>,
    pub invariant_report: Option<RenderInvariantReport>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderArtifactDimensions {
    pub width: f64,
    pub height: f64,
    pub view_box: Option<Rect>,
}

#[derive(Debug, Clone, Copy)]
pub enum RenderSceneContract<'a> {
    Typed(&'a RenderScene),
    NotMigrated,
    Unsupported,
    Inconsistent,
}

#[derive(Debug, Clone, Default)]
pub struct RenderInvariantReport {
    pub svg_violations: usize,
    pub typed_issues: Vec<GeometryIssue>,
    pub typed_metrics: Vec<GeometryMetric>,
    pub expansions: usize,
    pub background_rects_added: usize,
}

impl Default for RenderArtifact {
    fn default() -> Self {
        Self {
            svg: String::new(),
            format: BackendFormat::Svg,
            dimensions: None,
            diagnostics: Vec::new(),
            scene_availability: SceneAvailability::NotMigrated,
            scene: None,
            invariant_report: None,
        }
    }
}

impl RenderArtifact {
    pub fn svg_only(svg: String) -> Self {
        let mut artifact = Self {
            svg,
            format: BackendFormat::Svg,
            dimensions: None,
            diagnostics: Vec::new(),
            scene_availability: SceneAvailability::NotMigrated,
            scene: None,
            invariant_report: None,
        };
        artifact.refresh_svg_metadata();
        artifact
    }

    pub fn unsupported(svg: String) -> Self {
        Self::svg_only(svg).with_scene_availability(SceneAvailability::Unsupported)
    }

    pub fn with_scene(svg: String, scene: RenderScene) -> Self {
        let mut artifact = Self {
            svg,
            format: BackendFormat::Svg,
            dimensions: None,
            diagnostics: Vec::new(),
            scene_availability: SceneAvailability::TypedScene,
            scene: Some(scene),
            invariant_report: None,
        };
        artifact.refresh_svg_metadata();
        artifact
    }

    pub fn with_scene_availability(mut self, scene_availability: SceneAvailability) -> Self {
        if matches!(scene_availability, SceneAvailability::TypedScene) && self.scene.is_none() {
            self.scene_availability = SceneAvailability::NotMigrated;
        } else {
            self.scene_availability = scene_availability;
        }
        self
    }

    pub fn scene_contract(&self) -> RenderSceneContract<'_> {
        match (self.scene_availability, self.scene.as_ref()) {
            (SceneAvailability::TypedScene, Some(scene)) => RenderSceneContract::Typed(scene),
            (SceneAvailability::NotMigrated, None) => RenderSceneContract::NotMigrated,
            (SceneAvailability::Unsupported, None) => RenderSceneContract::Unsupported,
            _ => RenderSceneContract::Inconsistent,
        }
    }

    pub fn typed_scene(&self) -> Option<&RenderScene> {
        match self.scene_contract() {
            RenderSceneContract::Typed(scene) => Some(scene),
            _ => None,
        }
    }

    pub fn require_typed_scene(&self) -> Result<&RenderScene, Diagnostic> {
        match self.scene_contract() {
            RenderSceneContract::Typed(scene) => Ok(scene),
            RenderSceneContract::NotMigrated => Err(Diagnostic::error(
                "[E_RENDER_SCENE_NOT_MIGRATED] renderer has not been migrated to typed RenderScene",
            )),
            RenderSceneContract::Unsupported => Err(Diagnostic::error(
                "[E_RENDER_SCENE_UNSUPPORTED] renderer cannot expose typed RenderScene",
            )),
            RenderSceneContract::Inconsistent => Err(Diagnostic::error(
                "[E_RENDER_SCENE_CONTRACT] render artifact has inconsistent scene availability",
            )),
        }
    }

    pub fn media_type(&self) -> &'static str {
        self.format.media_type()
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn refresh_svg_metadata(&mut self) {
        self.dimensions = svg_dimensions(&self.svg);
    }
}

fn svg_dimensions(svg: &str) -> Option<RenderArtifactDimensions> {
    let width = svg_numeric_attr(svg, "width")?;
    let height = svg_numeric_attr(svg, "height")?;
    Some(RenderArtifactDimensions {
        width,
        height,
        view_box: svg_view_box(svg),
    })
}

fn svg_view_box(svg: &str) -> Option<Rect> {
    let start = svg.find("viewBox=\"")? + "viewBox=\"".len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    let values = rest[..end]
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if values.len() != 4 {
        return None;
    }
    Some(Rect::new(values[0], values[1], values[2], values[3]))
}

fn svg_numeric_attr(svg: &str, attr: &str) -> Option<f64> {
    let needle = format!("{attr}=\"");
    let start = svg.find(&needle)? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

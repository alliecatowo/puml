use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Fixture {
    pub(crate) path: String,
    pub(crate) family: String,
    pub(crate) expected_text: Vec<String>,
    #[serde(default)]
    pub(crate) unexpected_text: Vec<String>,
    pub(crate) min_text_elements: usize,
    #[serde(default)]
    pub(crate) structural_only_reason: Option<String>,
    #[serde(default)]
    pub(crate) required_classes: Vec<String>,
    #[serde(default)]
    pub(crate) expected_counts: BTreeMap<String, ExpectedCount>,
    #[serde(default)]
    pub(crate) required_data_attrs: Vec<DataAttrRequirement>,
    #[serde(default)]
    pub(crate) geometry_profile: Option<GeometryProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum DataAttrRequirement {
    Name(String),
    Match {
        name: String,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        class: Option<String>,
        #[serde(default)]
        tag: Option<String>,
    },
}

impl DataAttrRequirement {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Name(name) | Self::Match { name, .. } => name,
        }
    }

    pub(crate) fn description(&self) -> String {
        match self {
            Self::Name(name) => name.clone(),
            Self::Match {
                name,
                value,
                class,
                tag,
            } => {
                let mut parts = vec![name.clone()];
                if let Some(value) = value {
                    parts.push(format!("={value:?}"));
                }
                if let Some(class) = class {
                    parts.push(format!(" on .{class}"));
                }
                if let Some(tag) = tag {
                    parts.push(format!(" on <{tag}>"));
                }
                parts.join("")
            }
        }
    }

    pub(crate) fn matching_count(&self, doc: &crate::svg_test_helpers::SvgDoc<'_>) -> usize {
        match self {
            Self::Name(name) => doc.attr_count(name),
            Self::Match {
                name,
                value,
                class,
                tag,
            } => doc
                .elements_matching_attr(name, value.as_deref(), class.as_deref(), tag.as_deref())
                .len(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ExpectedCount {
    Exact(usize),
    Range {
        #[serde(default)]
        exact: Option<usize>,
        #[serde(default)]
        min: Option<usize>,
        #[serde(default)]
        max: Option<usize>,
    },
}

impl ExpectedCount {
    pub(crate) fn accepts(&self, actual: usize) -> bool {
        match self {
            Self::Exact(expected) => actual == *expected,
            Self::Range { exact, min, max } => {
                exact.is_none_or(|expected| actual == expected)
                    && min.is_none_or(|expected| actual >= expected)
                    && max.is_none_or(|expected| actual <= expected)
            }
        }
    }

    pub(crate) fn description(&self) -> String {
        match self {
            Self::Exact(expected) => format!("exactly {expected}"),
            Self::Range { exact, min, max } => {
                let mut parts = Vec::new();
                if let Some(exact) = exact {
                    parts.push(format!("exactly {exact}"));
                }
                if let Some(min) = min {
                    parts.push(format!("at least {min}"));
                }
                if let Some(max) = max {
                    parts.push(format!("at most {max}"));
                }
                parts.join(" and ")
            }
        }
    }

    fn is_well_formed(&self) -> bool {
        match self {
            Self::Exact(_) => true,
            Self::Range { exact, min, max } => {
                (exact.is_some() || min.is_some() || max.is_some())
                    && match (*min, *max) {
                        (Some(min), Some(max)) => min <= max,
                        _ => true,
                    }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeometryProfile {
    Basic,
    Graph,
    Chart,
    Tree,
    Timeline,
    StructuralOnly,
    Unsupported,
}

impl<'de> Deserialize<'de> for GeometryProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "basic" => Ok(Self::Basic),
            "graph" => Ok(Self::Graph),
            "chart" => Ok(Self::Chart),
            "tree" => Ok(Self::Tree),
            "timeline" => Ok(Self::Timeline),
            "structural-only" => Ok(Self::StructuralOnly),
            "unsupported" => Ok(Self::Unsupported),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "basic",
                    "graph",
                    "chart",
                    "tree",
                    "timeline",
                    "structural-only",
                    "unsupported",
                ],
            )),
        }
    }
}

pub(crate) fn load_manifest() -> Manifest {
    let raw = include_str!("manifest.json");
    serde_json::from_str(raw).expect("manifest.json must be valid JSON")
}

#[test]
fn manifest_requires_semantic_text_expectations_or_explicit_exception() {
    let manifest = load_manifest();
    let weak_fixtures = manifest
        .fixtures
        .iter()
        .filter(|fixture| {
            let has_exception = fixture
                .structural_only_reason
                .as_deref()
                .is_some_and(|reason| !reason.trim().is_empty());
            let has_blank_expected_text = fixture
                .expected_text
                .iter()
                .any(|expected| expected.trim().is_empty());

            has_blank_expected_text
                || (!has_exception
                    && (fixture.expected_text.is_empty() || fixture.min_text_elements == 0))
        })
        .map(|fixture| fixture.path.as_str())
        .collect::<Vec<_>>();

    assert!(
        weak_fixtures.is_empty(),
        "visual manifest fixtures must assert semantic expected_text and nonzero \
         min_text_elements, or include non-empty structural_only_reason for \
         machine/structural-only exceptions: {weak_fixtures:#?}"
    );
}

#[test]
fn manifest_semantic_svg_contract_fields_are_well_formed() {
    let manifest = load_manifest();
    let mut problems = Vec::new();

    for fixture in &manifest.fixtures {
        for class_name in &fixture.required_classes {
            if class_name.trim().is_empty() || class_name.split_whitespace().count() != 1 {
                problems.push(format!(
                    "{} has invalid required_classes entry {:?}",
                    fixture.path, class_name
                ));
            }
        }

        for attr in &fixture.required_data_attrs {
            let name = attr.name();
            if name.trim().is_empty() || !name.starts_with("data-") {
                problems.push(format!(
                    "{} has invalid required_data_attrs entry {}",
                    fixture.path,
                    attr.description()
                ));
            }
        }

        for (target, expected) in &fixture.expected_counts {
            if target.trim().is_empty() || target.split_whitespace().count() != 1 {
                problems.push(format!(
                    "{} has invalid expected_counts target {:?}",
                    fixture.path, target
                ));
            }
            if !expected.is_well_formed() {
                problems.push(format!(
                    "{} has invalid expected_counts expectation for {:?}",
                    fixture.path, target
                ));
            }
        }

        if matches!(
            fixture.geometry_profile,
            Some(GeometryProfile::StructuralOnly | GeometryProfile::Unsupported)
        ) && fixture
            .structural_only_reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            problems.push(format!(
                "{} uses an escape-hatch geometry profile without structural_only_reason",
                fixture.path
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "visual manifest semantic contract fields must be well formed: {problems:#?}"
    );
}

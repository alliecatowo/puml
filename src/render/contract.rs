//! Typed SVG emission contract.
//!
//! Renderer implementations still build family-specific SVG today, but raw SVG
//! should not cross the public render boundary unnoticed. This module gives
//! that boundary a name: renderers hand over `RawSvg`, the contract applies the
//! shared invariant pass, and callers receive an opaque `ValidatedSvg`.

use super::validate::{self, AutoCorrect, InvariantReport, InvariantViolation};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderProfile {
    Semantic,
    Graph,
    Chart,
    Tree,
    Timeline,
    Legacy,
}

impl RenderProfile {
    pub fn requires_semantic_hooks(self) -> bool {
        !matches!(self, Self::Legacy)
    }
}

impl fmt::Display for RenderProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Semantic => "semantic",
            Self::Graph => "graph",
            Self::Chart => "chart",
            Self::Tree => "tree",
            Self::Timeline => "timeline",
            Self::Legacy => "legacy",
        })
    }
}

#[derive(Debug, Clone)]
pub struct RawSvg {
    svg: String,
    profile: RenderProfile,
}

impl RawSvg {
    pub fn new(svg: String, profile: RenderProfile) -> Self {
        Self { svg, profile }
    }

    pub fn validate(mut self) -> Result<ValidatedSvg, SvgContractError> {
        let mut report = validate::run(&mut self.svg, AutoCorrect::Apply);
        let mut hard_violations = Vec::new();

        if !self.svg.trim_start().starts_with("<svg ") {
            hard_violations.push(ContractViolation::MalformedSvgRoot);
        }

        if self.profile.requires_semantic_hooks() {
            report
                .violations
                .extend(validate::expand_viewbox_to_semantic_bboxes(&mut self.svg));
            hard_violations.extend(
                validate::check_semantic_bboxes_inside_viewbox(&self.svg)
                    .into_iter()
                    .map(ContractViolation::Invariant),
            );
            hard_violations.extend(
                validate::check_canonical_semantic_hook_attrs(&self.svg)
                    .into_iter()
                    .map(ContractViolation::Invariant),
            );
        }

        if matches!(self.profile, RenderProfile::Graph) {
            report
                .violations
                .extend(validate::check_primary_node_non_overlap(&self.svg));
            report
                .violations
                .extend(validate::check_labels_clear_non_owner_nodes(&self.svg));
        }

        if hard_violations.is_empty() {
            Ok(ValidatedSvg {
                svg: self.svg,
                profile: self.profile,
                report,
            })
        } else {
            report.violations.extend(hard_violations.iter().filter_map(
                |violation| match violation {
                    ContractViolation::Invariant(invariant) => Some(invariant.clone()),
                    ContractViolation::MalformedSvgRoot => None,
                },
            ));
            Err(SvgContractError {
                profile: self.profile,
                violations: hard_violations,
                report,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedSvg {
    svg: String,
    profile: RenderProfile,
    report: InvariantReport,
}

impl ValidatedSvg {
    pub fn as_str(&self) -> &str {
        &self.svg
    }

    pub fn into_string(self) -> String {
        self.svg
    }

    pub fn profile(&self) -> RenderProfile {
        self.profile
    }

    pub fn invariant_report(&self) -> &InvariantReport {
        &self.report
    }
}

#[derive(Debug, Clone)]
pub struct SvgContractError {
    pub profile: RenderProfile,
    pub violations: Vec<ContractViolation>,
    pub report: InvariantReport,
}

impl fmt::Display for SvgContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "render contract failed for {} profile with {} violation(s)",
            self.profile,
            self.violations.len()
        )?;
        for violation in self.violations.iter().take(3) {
            write!(f, ": {violation}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SvgContractError {}

#[derive(Debug, Clone)]
pub enum ContractViolation {
    MalformedSvgRoot,
    Invariant(InvariantViolation),
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedSvgRoot => f.write_str("render output does not start with <svg"),
            Self::Invariant(violation) => write!(f, "{}", violation.message),
        }
    }
}

pub fn validate_svg(svg: String, profile: RenderProfile) -> Result<ValidatedSvg, SvgContractError> {
    RawSvg::new(svg, profile).validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_svg_is_opaque_and_returns_inner_string() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"></svg>"#;
        let validated = validate_svg(svg.to_string(), RenderProfile::Legacy).unwrap();
        assert_eq!(validated.profile(), RenderProfile::Legacy);
        assert!(validated.invariant_report().violations.is_empty());
        assert_eq!(validated.into_string(), svg);
    }

    #[test]
    fn graph_profile_rejects_missing_canonical_hooks() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect class="puml-node" data-puml-id="A" x="10" y="10" width="20" height="20"/></svg>"#;
        let err = validate_svg(svg.to_string(), RenderProfile::Graph).unwrap_err();
        assert!(err
            .violations
            .iter()
            .any(|violation| violation.to_string().contains("data-puml-bbox")));
    }
}

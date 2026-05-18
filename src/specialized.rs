/// Specialized diagram renderers for @startregex, @startebnf,
/// @startmath/@startlatex, and @startditaa diagram families.
///
/// These bypass the main AST parser pipeline and implement their own
/// mini-parsers and SVG renderers.
///
/// Note: @startchart is NOT handled here. It goes through the normal
/// AST parse → normalize → render_chart_svg pipeline.
mod ditaa;
mod ebnf;
mod math;
mod railroad;
mod regex;
mod shared;

use crate::diagnostic::Diagnostic;

use ditaa::render_ditaa;
use ebnf::render_ebnf;
use math::{render_latex, render_math};
use regex::render_regex;

pub(crate) use ditaa::render_ditaa_from_parts;
pub(crate) use math::render_math_from_parts;


/// Try to render `source` as one of the specialized diagram families.
/// Returns `Some(svg)` if the source is recognized, `None` otherwise.
pub fn try_render_specialized(source: &str) -> Option<Result<String, Diagnostic>> {
    let family = detect_specialized_family(source)?;
    Some(match family {
        SpecializedFamily::Regex => render_regex(source.trim()),
        SpecializedFamily::Ebnf => render_ebnf(source.trim()),
        SpecializedFamily::Math => render_math(source.trim()),
        SpecializedFamily::Latex => render_latex(source.trim()),
        SpecializedFamily::Ditaa => render_ditaa(source.trim()),
    })
}

pub fn is_specialized_source(source: &str) -> bool {
    detect_specialized_family(source).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpecializedFamily {
    Regex,
    Ebnf,
    Math,
    Latex,
    Ditaa,
}

fn detect_specialized_family(source: &str) -> Option<SpecializedFamily> {
    let trimmed = source.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("@startregex") {
        Some(SpecializedFamily::Regex)
    } else if lower.starts_with("@startebnf") {
        Some(SpecializedFamily::Ebnf)
    } else if lower.starts_with("@startmath") {
        Some(SpecializedFamily::Math)
    } else if lower.starts_with("@startlatex") {
        Some(SpecializedFamily::Latex)
    } else if lower.starts_with("@startditaa") {
        Some(SpecializedFamily::Ditaa)
    } else {
        None
    }
}

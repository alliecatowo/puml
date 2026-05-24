use super::*;

pub(super) fn handle_family_overflow_skinparam(
    key: &str,
    value: &str,
    policy: &mut TextOverflowPolicy,
    warnings: &mut Vec<Diagnostic>,
    span: crate::source::Span,
) -> bool {
    let normalized_key = key.trim().to_ascii_lowercase();
    let normalized_value = value.trim().to_ascii_lowercase();
    if normalized_key != "textoverflowpolicy" && normalized_key != "text_overflow_policy" {
        return false;
    }

    let parsed = match normalized_value.as_str() {
        "wrap" | "wrapandgrow" | "wrap_and_grow" | "wrapgrow" => {
            Some(TextOverflowPolicy::WrapAndGrow)
        }
        "ellipsis" | "ellipsesingleline" | "ellipsissingleline" | "singleline" | "nowrap" => {
            Some(TextOverflowPolicy::EllipsisSingleLine)
        }
        _ => {
            warnings.push(
                Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                    value, key
                ))
                .with_span(span),
            );
            None
        }
    };
    if let Some(parsed) = parsed {
        *policy = parsed;
    }
    true
}

pub(super) fn parse_family_orientation_directive(line: &str) -> Option<FamilyOrientation> {
    let tokens = line
        .split_whitespace()
        .map(|t| t.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens.len() == 4 && tokens[3].as_str() == "direction" {
        let key = [&tokens[0][..], &tokens[1][..], &tokens[2][..]].join(" ");
        return match key.as_str() {
            "left to right" => Some(FamilyOrientation::LeftToRight),
            "right to left" => Some(FamilyOrientation::RightToLeft),
            "top to bottom" => Some(FamilyOrientation::TopToBottom),
            "bottom to top" => Some(FamilyOrientation::BottomToTop),
            _ => None,
        };
    }
    None
}

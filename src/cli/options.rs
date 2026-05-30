use clap::ValueEnum;

/// Parse a single `-DKEY=VALUE` or `-D KEY=VALUE` argument into `(key, value)`.
/// A bare key with no `=` is accepted and yields an empty value.
pub fn parse_define(raw: &str) -> Result<(String, String), String> {
    match raw.split_once('=') {
        Some((key, val)) => {
            let key = key.trim().to_string();
            if key.is_empty() {
                return Err(format!("variable name cannot be empty in '-D{raw}'"));
            }
            Ok((key, val.to_string()))
        }
        None => {
            let key = raw.trim().to_string();
            if key.is_empty() {
                return Err("variable name cannot be empty in '-D' flag".to_string());
            }
            Ok((key, String::new()))
        }
    }
}

pub fn parse_dpi(raw: &str) -> Result<f32, String> {
    let value = raw
        .parse::<f32>()
        .map_err(|e| format!("invalid DPI '{raw}': {e}"))?;
    if (1.0..=1200.0).contains(&value) {
        Ok(value)
    } else {
        Err("dpi must be in range [1, 1200]".to_string())
    }
}

pub fn parse_threads(raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|e| format!("invalid thread count '{raw}': {e}"))?;
    if value > 0 {
        Ok(value)
    } else {
        Err("thread count must be at least 1".to_string())
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DiagnosticsFormat {
    Human,
    Json,
    Stdrpt,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DumpKind {
    Ast,
    Model,
    Scene,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum LintReportFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum Dialect {
    Auto,
    Plantuml,
    Mermaid,
    Picouml,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum CompatMode {
    Strict,
    Extended,
}

/// Chrome rendering mode for diagram output.
///
/// `Puml` (default) enables PUML-native chrome enhancements:
/// rich header fills, class/object type badges, UML 2.x visibility
/// glyphs, and drop shadows when shadowing is on.
///
/// `Plantuml` suppresses PUML-specific chrome so diagrams visually
/// match PlantUML's neutral default look: flat gray object headers,
/// no type badges, ASCII visibility prefixes.
/// **Layout is always identical between modes** — only paint differs.
#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq, Default)]
pub enum StyleMode {
    /// PUML-enhanced chrome (default): richer fills, badges, UML glyphs.
    #[default]
    Puml,
    /// PlantUML-compatible neutral chrome: flat fills, no badges, ASCII visibility.
    Plantuml,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_parser_accepts_boundaries_and_rejects_invalid_values() {
        assert_eq!(parse_dpi("1").expect("lower-bound DPI should parse"), 1.0);
        assert_eq!(
            parse_dpi("1200").expect("upper-bound DPI should parse"),
            1200.0
        );
        assert!(parse_dpi("0.999").is_err());
        assert!(parse_dpi("1200.1").is_err());
        assert!(parse_dpi("not-a-number").is_err());
    }

    #[test]
    fn define_parser_accepts_key_values_and_rejects_empty_keys() {
        assert_eq!(
            parse_define("COLOR=red").expect("key/value define should parse"),
            ("COLOR".to_string(), "red".to_string())
        );
        assert_eq!(
            parse_define(" DEBUG ").expect("bare define should parse"),
            ("DEBUG".to_string(), String::new())
        );
        assert!(parse_define("=bad").is_err());
        assert!(parse_define("").is_err());
    }

    #[test]
    fn threads_parser_accepts_positive_values_only() {
        assert_eq!(parse_threads("1").expect("one thread should parse"), 1);
        assert_eq!(parse_threads("16").expect("many threads should parse"), 16);
        assert!(parse_threads("0").is_err());
        assert!(parse_threads("not-a-number").is_err());
    }
}

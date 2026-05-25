use std::collections::BTreeMap;

use crate::ast::{DiagramKind, Document, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::model::{
    WireComponent, WireDocument, WireEndpoint, WireLabel, WireLink, WirePort, WirePortSide,
};

const DEFAULT_COMPONENT_WIDTH: f64 = 120.0;
const DEFAULT_COMPONENT_HEIGHT: f64 = 72.0;
const DEFAULT_SPOT_WIDTH: f64 = 72.0;
const DEFAULT_SPOT_HEIGHT: f64 = 34.0;
const ROW_GAP: f64 = 34.0;
const COLUMN_GAP: f64 = 90.0;
const ORIGIN_X: f64 = 32.0;
const ORIGIN_Y: f64 = 42.0;

#[derive(Debug, Default)]
struct WireCursor {
    x: f64,
    y: f64,
    column_width: f64,
    current_component: Option<String>,
    vars: BTreeMap<String, String>,
    label_count: usize,
    link_count: usize,
}

impl WireCursor {
    fn new() -> Self {
        Self {
            x: ORIGIN_X,
            y: ORIGIN_Y,
            column_width: DEFAULT_COMPONENT_WIDTH,
            current_component: None,
            vars: BTreeMap::new(),
            label_count: 0,
            link_count: 0,
        }
    }

    fn new_column(&mut self) {
        self.x += self.column_width + COLUMN_GAP;
        self.y = ORIGIN_Y;
        self.column_width = DEFAULT_COMPONENT_WIDTH;
        self.current_component = None;
    }

    fn add_vspace(&mut self, value: f64) {
        self.y += value.max(0.0);
    }

    fn after_component(&mut self, id: String, height: f64, width: f64) {
        self.current_component = Some(id);
        self.y += height + ROW_GAP;
        self.column_width = self.column_width.max(width);
    }
}

pub(super) fn normalize_wire(document: Document) -> Result<WireDocument, Diagnostic> {
    if document.kind != DiagramKind::Wire {
        return Err(Diagnostic::error(
            "[E_WIRE_INTERNAL] invalid family for wire normalization",
        ));
    }

    let mut out = WireDocument::default();
    let mut cursor = WireCursor::new();
    let mut warnings = Vec::new();

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::RawBody(line) => {
                normalize_wire_line(&line, stmt.span, &mut cursor, &mut out, &mut warnings)?;
            }
            StatementKind::Title(value) => out.title = Some(value),
            StatementKind::Header(value) => out.header = Some(value),
            StatementKind::Footer(value) => out.footer = Some(value),
            StatementKind::Caption(value) => out.caption = Some(value),
            StatementKind::Legend(value) => out.legend = Some(value),
            StatementKind::SkinParam { key, value } => warnings.push(
                Diagnostic::warning(format!(
                    "[W_WIRE_SKINPARAM_UNSUPPORTED] wire renderer ignores skinparam `{key} {value}`"
                ))
                .with_span(stmt.span),
            ),
            StatementKind::Theme(value) => warnings.push(
                Diagnostic::warning(format!(
                    "[W_WIRE_THEME_UNSUPPORTED] wire renderer ignores theme `{value}`"
                ))
                .with_span(stmt.span),
            ),
            StatementKind::Pragma(_) | StatementKind::Include(_) => {}
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                warnings.push(
                    Diagnostic::warning(format!(
                        "[W_WIRE_UNSUPPORTED] unsupported wire syntax ignored: `{}`",
                        raw.line
                    ))
                    .with_span(stmt.span),
                );
            }
            _ => {
                warnings.push(
                    Diagnostic::warning(
                        "[W_WIRE_UNSUPPORTED] unsupported statement ignored in wire diagram",
                    )
                    .with_span(stmt.span),
                );
            }
        }
    }

    out.warnings = warnings;
    Ok(out)
}

fn normalize_wire_line(
    raw_line: &str,
    span: crate::source::Span,
    cursor: &mut WireCursor,
    out: &mut WireDocument,
    warnings: &mut Vec<Diagnostic>,
) -> Result<(), Diagnostic> {
    let expanded = expand_vars(raw_line.trim(), &cursor.vars);
    let line = expanded.trim();
    if line.is_empty() {
        return Ok(());
    }
    if let Some((name, value)) = parse_variable_assignment(line) {
        cursor.vars.insert(name.to_string(), value.to_string());
        return Ok(());
    }
    if parse_metadata_line(line, out) {
        return Ok(());
    }
    if let Some(amount) = parse_number_after_keyword(line, "vspace") {
        cursor.add_vspace(amount);
        return Ok(());
    }
    if let Some((x, y)) = parse_point_after_keyword(line, "move") {
        cursor.x += x;
        cursor.y += y;
        return Ok(());
    }
    if let Some((x, y)) = parse_point_after_keyword(line, "goto") {
        cursor.x = x;
        cursor.y = y;
        return Ok(());
    }
    if line.eq_ignore_ascii_case("--") || line.eq_ignore_ascii_case("right") {
        cursor.new_column();
        return Ok(());
    }
    if let Some(text) = parse_print(line) {
        cursor.label_count += 1;
        out.labels.push(WireLabel {
            id: format!("wire-label-{}", cursor.label_count),
            text,
            x: cursor.x,
            y: cursor.y,
        });
        cursor.y += 22.0;
        return Ok(());
    }
    if let Some(link) = parse_link_line(line, cursor) {
        out.links.push(link);
        return Ok(());
    }

    let mut handled_any = false;
    for (idx, segment) in line.split(" -- ").enumerate() {
        let segment = segment.trim();
        if idx > 0 {
            cursor.new_column();
        }
        if segment.is_empty() {
            continue;
        }
        if let Some(component) = parse_component_segment(segment, cursor)? {
            cursor.after_component(component.id.clone(), component.height, component.width);
            out.components.push(component);
            handled_any = true;
            continue;
        }
        if apply_port_directive(segment, cursor, out) {
            handled_any = true;
            continue;
        }
        warnings.push(
            Diagnostic::warning(format!(
                "[W_WIRE_UNSUPPORTED] unsupported wire syntax ignored: `{segment}`"
            ))
            .with_span(span),
        );
    }

    if handled_any {
        Ok(())
    } else {
        Err(Diagnostic::error(format!(
            "[E_WIRE_EMPTY_LINE] wire line did not contain a supported command: `{line}`"
        ))
        .with_span(span))
    }
}

fn parse_variable_assignment(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("!$")?;
    let (name, value) = rest.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    Some((name, value))
}

fn expand_vars(line: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = line.to_string();
    for (name, value) in vars {
        out = out.replace(&format!("${name}"), value);
    }
    out
}

fn parse_metadata_line(line: &str, out: &mut WireDocument) -> bool {
    let Some((key, value)) = line.split_once(char::is_whitespace) else {
        return false;
    };
    let value = value.trim();
    match key.to_ascii_lowercase().as_str() {
        "title" => out.title = Some(value.to_string()),
        "header" => out.header = Some(value.trim_start_matches(':').trim().to_string()),
        "footer" => out.footer = Some(value.trim_start_matches(':').trim().to_string()),
        "caption" => out.caption = Some(value.to_string()),
        "legend" => out.legend = Some(value.to_string()),
        _ => return false,
    }
    true
}

fn parse_number_after_keyword(line: &str, keyword: &str) -> Option<f64> {
    let rest = line.strip_prefix(keyword)?.trim();
    rest.parse::<f64>().ok()
}

fn parse_point_after_keyword(line: &str, keyword: &str) -> Option<(f64, f64)> {
    let rest = line.strip_prefix(keyword)?.trim();
    let rest = rest
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix(')'))
        .unwrap_or(rest);
    let (x, y) = rest.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn parse_print(line: &str) -> Option<String> {
    let rest = line.strip_prefix("print(")?.strip_suffix(')')?.trim();
    Some(unquote(rest).replace("\\n", "\n"))
}

fn parse_component_segment(
    segment: &str,
    cursor: &WireCursor,
) -> Result<Option<WireComponent>, Diagnostic> {
    let trimmed = segment.trim();
    let (after_keyword, spot) = if let Some(rest) = trimmed.strip_prefix("component ") {
        (rest.trim(), false)
    } else if let Some(rest) = trimmed.strip_prefix('*') {
        (rest.trim(), false)
    } else if trimmed.starts_with('$') {
        (trimmed, true)
    } else {
        return Ok(None);
    };

    let (head, rest) = split_at_size(after_keyword);
    let name = head.split_whitespace().next().unwrap_or(head).trim();
    if name.is_empty() {
        return Err(Diagnostic::error(
            "[E_WIRE_COMPONENT_NAME] wire component command requires a name",
        ));
    }
    let (width, height) = parse_size(rest).unwrap_or(if spot {
        (DEFAULT_SPOT_WIDTH, DEFAULT_SPOT_HEIGHT)
    } else {
        (DEFAULT_COMPONENT_WIDTH, DEFAULT_COMPONENT_HEIGHT)
    });
    let id = component_id(name);
    let mut component = WireComponent {
        id: id.clone(),
        label: name.trim_start_matches('$').to_string(),
        x: cursor.x,
        y: cursor.y,
        width,
        height,
        color: parse_color(rest),
        ports: Vec::new(),
    };
    apply_port_specs(rest, &mut component);
    Ok(Some(component))
}

fn split_at_size(input: &str) -> (&str, &str) {
    if let Some(idx) = input.find('[') {
        (&input[..idx], &input[idx..])
    } else {
        (input, "")
    }
}

fn parse_size(rest: &str) -> Option<(f64, f64)> {
    let open = rest.find('[')?;
    let close = rest[open + 1..].find(']')? + open + 1;
    let body = &rest[open + 1..close];
    let (w, h) = body.split_once('x').or_else(|| body.split_once('X'))?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn parse_color(rest: &str) -> Option<String> {
    rest.split_whitespace()
        .find(|token| token.starts_with('#') && token.len() > 1)
        .map(ToString::to_string)
}

fn apply_port_directive(segment: &str, cursor: &WireCursor, out: &mut WireDocument) -> bool {
    let Some(component_id) = cursor.current_component.as_deref() else {
        return false;
    };
    let Some(component) = out.components.iter_mut().find(|c| c.id == component_id) else {
        return false;
    };
    let before = component.ports.len();
    apply_port_specs(segment, component);
    component.ports.len() > before
}

fn apply_port_specs(text: &str, component: &mut WireComponent) {
    for side in [
        WirePortSide::Left,
        WirePortSide::Right,
        WirePortSide::Top,
        WirePortSide::Bottom,
    ] {
        let needle = format!("{}:", side.as_str());
        let mut rest = text;
        while let Some(idx) = rest.find(&needle) {
            let after = &rest[idx + needle.len()..];
            let value_end = next_port_spec_offset(after).unwrap_or(after.len());
            add_ports(component, side, &after[..value_end]);
            rest = &after[value_end..];
        }
    }
}

fn next_port_spec_offset(text: &str) -> Option<usize> {
    [" left:", " right:", " top:", " bottom:"]
        .iter()
        .filter_map(|needle| text.find(needle))
        .min()
}

fn add_ports(component: &mut WireComponent, side: WirePortSide, raw: &str) {
    for label in raw.split(',').map(str::trim).filter(|v| !v.is_empty()) {
        let order = component
            .ports
            .iter()
            .filter(|port| port.side == side)
            .count();
        let clean = label
            .split_whitespace()
            .next()
            .unwrap_or(label)
            .trim_matches('"')
            .to_string();
        if clean.is_empty() {
            continue;
        }
        component.ports.push(WirePort {
            id: format!("{}:{}:{}", component.id, side.as_str(), clean),
            label: clean,
            side,
            order,
        });
    }
}

fn parse_link_line(line: &str, cursor: &mut WireCursor) -> Option<WireLink> {
    let (left, rest, directed) = if let Some((left, rest)) = line.split_once("-->") {
        (left, rest, true)
    } else if let Some((left, rest)) = line.split_once("->") {
        (left, rest, true)
    } else if let Some((left, rest)) = line.split_once("--") {
        (left, rest, false)
    } else {
        return None;
    };
    let (right, label) = split_label(rest);
    let from = parse_endpoint(left.trim())?;
    let to = parse_endpoint(right.trim())?;
    cursor.link_count += 1;
    Some(WireLink {
        id: format!("wire-link-{}", cursor.link_count),
        from,
        to,
        label,
        directed,
    })
}

fn split_label(rest: &str) -> (&str, Option<String>) {
    if let Some((target, label)) = rest.split_once(':') {
        (target.trim(), Some(label.trim().to_string()))
    } else {
        (rest.trim(), None)
    }
}

fn parse_endpoint(raw: &str) -> Option<WireEndpoint> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((component, port)) = raw.split_once('.') {
        return Some(WireEndpoint {
            component: component_id(component.trim()),
            port: Some(port.trim().to_string()),
        });
    }
    if let Some((component, port)) = raw.split_once(':') {
        return Some(WireEndpoint {
            component: component_id(component.trim()),
            port: Some(port.trim().to_string()),
        });
    }
    Some(WireEndpoint {
        component: component_id(raw),
        port: None,
    })
}

fn component_id(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_start_matches('$')
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn unquote(raw: &str) -> String {
    raw.strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| raw.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(raw)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn wire_normalizer_builds_components_ports_and_links() {
        let src = "@startwire\ncomponent Panel [100x80] right:A,B\n--\ncomponent FPGA [140x90] left:A,B\nPanel.A -- FPGA.A : bus\n@endwire\n";
        let doc = parser::parse(src).expect("parse wire");
        let wire = normalize_wire(doc).expect("normalize wire");

        assert_eq!(wire.components.len(), 2);
        assert_eq!(wire.components[0].ports.len(), 2);
        assert_eq!(wire.components[1].x, 242.0);
        assert_eq!(wire.links.len(), 1);
        assert_eq!(wire.links[0].label.as_deref(), Some("bus"));
    }

    #[test]
    fn wire_normalizer_expands_simple_variables_in_sizes() {
        let src = "@startwire\n!$SwitchWidth=100\n*Main_Switch [$SwitchWidthx30]\n@endwire\n";
        let doc = parser::parse(src).expect("parse wire variable");
        let wire = normalize_wire(doc).expect("normalize wire variable");

        assert_eq!(wire.components[0].width, 100.0);
        assert_eq!(wire.components[0].height, 30.0);
    }
}

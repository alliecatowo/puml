use crate::ast::MemberModifier;
use crate::model::FamilyNodeKind;
use crate::render::svg::escape_text;

pub(super) fn parse_visibility_member(member: &str) -> (Option<&'static str>, &'static str, &str) {
    let trimmed = member.trim();
    if let Some(rest) = trimmed.strip_prefix('\\') {
        if matches!(rest.chars().next(), Some('+' | '-' | '#' | '~')) {
            return (None, "#334155", rest);
        }
    }
    match trimmed.chars().next() {
        Some('+') => (Some("+"), "#16a34a", trimmed[1..].trim_start()),
        Some('-') => (Some("-"), "#dc2626", trimmed[1..].trim_start()),
        Some('#') => (Some("#"), "#d97706", trimmed[1..].trim_start()),
        Some('~') => (Some("~"), "#7c3aed", trimmed[1..].trim_start()),
        _ => (None, "#334155", trimmed),
    }
}

pub(super) fn uml_visibility_name(symbol: &str) -> &'static str {
    match symbol {
        "+" => "public",
        "-" => "private",
        "#" => "protected",
        "~" => "package",
        _ => "unknown",
    }
}

pub(super) fn member_modifier_name(modifier: Option<&MemberModifier>) -> Option<&'static str> {
    match modifier {
        Some(MemberModifier::Field) => Some("field"),
        Some(MemberModifier::Method) => Some("method"),
        Some(MemberModifier::Abstract) => Some("abstract"),
        Some(MemberModifier::Static) => Some("static"),
        None => None,
    }
}

/// Detect if a member text is a compartment divider (PlantUML 3.8).
///
/// Bare dividers: `--`, `..`, `==`, `__` → `Some(None)` (divider with no title).
/// Titled dividers: `-- Section Name --`, `== Title ==`, `__ sub __`, `.. note ..`
///   → `Some(Some(&str))` where the inner str is the title.
///
/// Returns `None` if the text is not a divider.
pub(super) fn parse_member_divider(text: &str) -> Option<Option<&str>> {
    let t = text.trim();
    for delim in ["--", "..", "==", "__"] {
        if t == delim {
            return Some(None);
        }
        if t.starts_with(delim) && t.ends_with(delim) && t.len() > delim.len() * 2 {
            let inner = &t[delim.len()..t.len() - delim.len()];
            let title = inner.trim();
            if !title.is_empty() {
                return Some(Some(title));
            }
            // only delimiters, e.g. `----` or `====`
            return Some(None);
        }
    }
    None
}

/// Parse {abstract} / {static} modifiers from member text.
/// Returns (SVG style attrs string, cleaned text without modifiers).
pub(super) fn parse_member_modifiers(text: &str) -> (&'static str, &str) {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix("{abstract}") {
        (" font-style=\"italic\"", rest.trim_start())
    } else if let Some(rest) = t.strip_prefix("{static}") {
        (" text-decoration=\"underline\"", rest.trim_start())
    } else {
        ("", t)
    }
}

pub(crate) fn family_node_label(kind: FamilyNodeKind) -> &'static str {
    if let Some(spec) = crate::registry::graph_element_for_family_node_kind(kind) {
        return spec.renderer_label;
    }
    match kind {
        FamilyNodeKind::Class => "class",
        FamilyNodeKind::Object => "object",
        FamilyNodeKind::Map => "map",
        FamilyNodeKind::Diamond => "diamond",
        FamilyNodeKind::UseCase => "usecase",
        FamilyNodeKind::Salt => "widget",
        FamilyNodeKind::MindMap => "mindmap",
        FamilyNodeKind::Wbs => "wbs",
        FamilyNodeKind::Component => "component",
        FamilyNodeKind::Interface => "interface",
        FamilyNodeKind::Port => "port",
        FamilyNodeKind::Action => "action",
        FamilyNodeKind::Agent => "agent",
        FamilyNodeKind::Node => "node",
        FamilyNodeKind::Artifact => "artifact",
        FamilyNodeKind::Boundary => "boundary",
        FamilyNodeKind::Cloud => "cloud",
        FamilyNodeKind::Circle => "circle",
        FamilyNodeKind::Collections => "collections",
        FamilyNodeKind::Frame => "frame",
        FamilyNodeKind::Storage => "storage",
        FamilyNodeKind::Container => "container",
        FamilyNodeKind::Control => "control",
        FamilyNodeKind::Database => "database",
        FamilyNodeKind::Entity => "entity",
        FamilyNodeKind::Package => "package",
        FamilyNodeKind::Rectangle => "rectangle",
        FamilyNodeKind::Folder => "folder",
        FamilyNodeKind::File => "file",
        FamilyNodeKind::Card => "card",
        FamilyNodeKind::Actor => "actor",
        FamilyNodeKind::BusinessActor => "business-actor",
        FamilyNodeKind::BusinessUseCase => "business-usecase",
        FamilyNodeKind::Hexagon => "hexagon",
        FamilyNodeKind::Label => "label",
        FamilyNodeKind::Person => "person",
        FamilyNodeKind::Process => "process",
        FamilyNodeKind::Queue => "queue",
        FamilyNodeKind::Stack => "stack",
        FamilyNodeKind::UseCaseDeployment => "usecase",
        FamilyNodeKind::State => "state",
        FamilyNodeKind::StateInitial => "initial",
        FamilyNodeKind::StateFinal => "final",
        FamilyNodeKind::StateHistory => "history",
        FamilyNodeKind::ActivityStart => "start",
        FamilyNodeKind::ActivityStop => "stop",
        FamilyNodeKind::ActivityAction => "action",
        FamilyNodeKind::ActivityDecision => "decision",
        FamilyNodeKind::ActivityFork => "fork",
        FamilyNodeKind::ActivityForkEnd => "end fork",
        FamilyNodeKind::ActivityMerge => "merge",
        FamilyNodeKind::ActivityPartition => "partition",
        FamilyNodeKind::TimingConcise => "concise",
        FamilyNodeKind::TimingRobust => "robust",
        FamilyNodeKind::TimingClock => "clock",
        FamilyNodeKind::TimingBinary => "binary",
        FamilyNodeKind::TimingEvent => "event",
        FamilyNodeKind::Note => "note",
        // C4 family
        FamilyNodeKind::C4Person => "person",
        FamilyNodeKind::C4PersonExt => "person_ext",
        FamilyNodeKind::C4System => "system",
        FamilyNodeKind::C4SystemExt => "system_ext",
        FamilyNodeKind::C4SystemDb => "system_db",
        FamilyNodeKind::C4SystemQueue => "system_queue",
        FamilyNodeKind::C4Container => "container",
        FamilyNodeKind::C4ContainerExt => "container_ext",
        FamilyNodeKind::C4ContainerDb => "container_db",
        FamilyNodeKind::C4ContainerQueue => "container_queue",
        FamilyNodeKind::C4Component => "component",
        FamilyNodeKind::C4ComponentExt => "component_ext",
        FamilyNodeKind::C4ComponentDb => "component_db",
        FamilyNodeKind::C4ComponentQueue => "component_queue",
        FamilyNodeKind::C4Boundary => "boundary",
    }
}

pub(super) fn builtin_type_stereotype_label(text: &str) -> Option<&'static str> {
    match text {
        "<<enum>>" | "<<enumeration>>" => Some("\u{ab}enumeration\u{bb}"),
        "<<interface>>" => Some("\u{ab}interface\u{bb}"),
        "<<abstract>>" | "<<abstract class>>" => Some("\u{ab}abstract\u{bb}"),
        "<<annotation>>" => Some("\u{ab}annotation\u{bb}"),
        "<<protocol>>" => Some("\u{ab}protocol\u{bb}"),
        "<<struct>>" => Some("\u{ab}struct\u{bb}"),
        // IE/ER entity — class-family rounded-rectangle with entity stereotype header.
        "<<entity>>" => Some("\u{ab}entity\u{bb}"),
        // Exception — PlantUML renders with a reddish header.
        "<<exception>>" => Some("\u{ab}exception\u{bb}"),
        // Metaclass, stereotype, circle — PlantUML built-in type keywords.
        "<<metaclass>>" => Some("\u{ab}metaclass\u{bb}"),
        "<<stereotype>>" => Some("\u{ab}stereotype\u{bb}"),
        "<<circle>>" => Some("\u{ab}circle\u{bb}"),
        // ── Smart-default DDD / architectural stereotype shapes (issue #1285) ────────
        // These are PUML extensions; they produce a canonical guillemet label AND a
        // visually distinct shape + header colour.  The skinparam cascade still wins
        // over the smart defaults (checked in class_node_render.rs).
        "<<controller>>" => Some("\u{ab}controller\u{bb}"),
        "<<service>>" => Some("\u{ab}service\u{bb}"),
        "<<repository>>" => Some("\u{ab}repository\u{bb}"),
        "<<value>>" => Some("\u{ab}value\u{bb}"),
        "<<aggregate>>" => Some("\u{ab}aggregate\u{bb}"),
        "<<factory>>" => Some("\u{ab}factory\u{bb}"),
        "<<datatype>>" => Some("\u{ab}datatype\u{bb}"),
        "<<utility>>" => Some("\u{ab}utility\u{bb}"),
        _ => None,
    }
}

/// Return true if `text` is an arbitrary user-defined stereotype marker
/// (any `<<…>>` value that is NOT one of the built-in type keywords and NOT a
/// spot stereotype encoded form).
pub(super) fn is_user_stereotype(text: &str) -> bool {
    text.starts_with("<<")
        && text.ends_with(">>")
        && builtin_type_stereotype_label(text).is_none()
        && parse_spot_member(text).is_none()
}

/// Parse a spot-stereotype member encoded as `<<spot:L:#color:Label>>`.
/// Returns `(letter_char, color_str, label_str)` on match, `None` otherwise.
///
/// This is the renderer-side counterpart of `parse_spot_stereotype` in the parser.
/// The parser emits `<<spot:L:#color:Label>>` (label may be empty) and this function
/// decodes it back so the renderer can draw the colored badge.
pub(super) fn parse_spot_member(text: &str) -> Option<(char, String, String)> {
    // Expected form: <<spot:L:#color:Label>> where label may be empty.
    let inner = text.strip_prefix("<<spot:")?.strip_suffix(">>")?;
    // First colon separates letter from the rest.
    let (letter_str, rest) = inner.split_once(':')?;
    let letter = letter_str.chars().next()?;
    if letter_str.len() != letter.len_utf8() {
        // More than one char before the colon — not a valid spot member.
        return None;
    }
    // Second colon separates color from label.
    let (color, label) = rest.split_once(':')?;
    if !color.starts_with('#') {
        return None;
    }
    Some((letter, color.to_string(), label.to_string()))
}

/// Count how many leading members of `members` are header stereotypes that
/// should be rendered in the class-box header rather than as member rows.
/// This includes the optional built-in type marker (first position) plus any
/// consecutive user-defined or spot stereotype markers that immediately follow it.
pub(super) fn count_header_stereotype_members(members: &[crate::ast::ClassMember]) -> usize {
    let mut skip = 0;
    // First member may be a built-in type marker (e.g. <<enum>>).
    if members
        .first()
        .is_some_and(|m| builtin_type_stereotype_label(&m.text).is_some())
    {
        skip += 1;
    }
    // Any consecutive user-defined <<…>> or spot <<spot:…>> members directly after
    // the type marker (or at the start if there was no type marker) are also header
    // stereotypes.
    while skip < members.len()
        && (is_user_stereotype(&members[skip].text)
            || parse_spot_member(&members[skip].text).is_some())
    {
        skip += 1;
    }
    skip
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MapRow<'a> {
    pub(super) key: &'a str,
    pub(super) value: &'a str,
}

pub(super) fn parse_map_row(text: &str) -> Option<MapRow<'_>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    for sep in ["<=>", "=>"] {
        if let Some((key, value)) = trimmed.split_once(sep) {
            return Some(MapRow {
                key: key.trim(),
                value: value.trim(),
            });
        }
    }
    for marker in [
        "*--->", "*-->", "*---", "*--", "*->", "-->", "---", "--", "..>", "...", "..",
    ] {
        if let Some((lhs, rhs)) = trimmed.split_once(marker) {
            return Some(MapRow {
                key: lhs.trim(),
                value: rhs.trim(),
            });
        }
    }
    Some(MapRow {
        key: trimmed,
        value: "",
    })
}

pub(super) struct MapRenderCtx<'a> {
    pub(super) font_family: &'a str,
    pub(super) member_font_size: u32,
    pub(super) member_color: &'a str,
    pub(super) stroke: &'a str,
}

pub(super) fn render_map_rows(
    out: &mut String,
    node: &crate::model::FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    header_h: i32,
    ctx: &MapRenderCtx<'_>,
) {
    let divider_x = x + (w * 45 / 100);
    let rows: Vec<_> = node
        .members
        .iter()
        .filter_map(|member| parse_map_row(&member.text))
        .collect();
    if rows.is_empty() {
        return;
    }
    out.push_str(&format!(
        "<line class=\"uml-map-divider\" x1=\"{divider_x}\" y1=\"{}\" x2=\"{divider_x}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        y + header_h,
        y + header_h + rows.len() as i32 * 18,
        ctx.stroke
    ));
    for (idx, row) in rows.iter().enumerate() {
        let row_top = y + header_h + idx as i32 * 18;
        let text_y = row_top + 12;
        if idx > 0 {
            out.push_str(&format!(
                "<line class=\"uml-map-row\" x1=\"{x}\" y1=\"{row_top}\" x2=\"{}\" y2=\"{row_top}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + w,
                ctx.stroke
            ));
        }
        let anchor = format!(
            "{}::{}",
            node.alias.as_deref().unwrap_or(&node.name),
            row.key
        );
        out.push_str(&format!(
            "<text class=\"uml-map-key\" data-uml-anchor=\"{}\" x=\"{}\" y=\"{text_y}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            escape_text(&anchor),
            x + 10,
            escape_text(ctx.font_family),
            ctx.member_font_size,
            escape_text(ctx.member_color),
            escape_text(row.key)
        ));
        out.push_str(&format!(
            "<text class=\"uml-map-value\" x=\"{}\" y=\"{text_y}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            divider_x + 10,
            escape_text(ctx.font_family),
            ctx.member_font_size,
            escape_text(ctx.member_color),
            escape_text(row.value)
        ));
    }
}

pub(super) fn is_family_style_member(text: &str) -> bool {
    text.starts_with("\x1fstyle:")
        || text.starts_with("\x1fclass:")
        || text.starts_with("\x1ffamily:tag:")
        || text.starts_with("\x1fuc:")
}

pub(super) fn class_node_visibility_symbol(
    node: &crate::model::FamilyNode,
) -> Option<&'static str> {
    node.members.iter().find_map(|member| {
        let symbol = member.text.strip_prefix("\x1fclass:visibility:")?;
        match symbol.trim() {
            "+" => Some("+"),
            "-" => Some("-"),
            "#" => Some("#"),
            "~" => Some("~"),
            _ => None,
        }
    })
}

/// Emit a UML 2.x visibility glyph SVG shape before the member text (#1349).
///
/// Returns the x-offset to add to the text position (14 when a glyph is emitted, 0 otherwise).
///
/// Hollow vs filled encodes field vs method:
/// - method: `is_method == true` (modifier == Method or text contains '(')
/// - field: everything else
///   Shapes: public=circle, private/protected=diamond, package=triangle.
pub(super) fn emit_visibility_glyph(
    out: &mut String,
    vis_sym: Option<&str>,
    color: &str,
    gx: i32,
    gy: i32,
    is_method: bool,
) -> i32 {
    let Some(sym) = vis_sym else {
        return 0;
    };
    match sym {
        "+" => {
            // Public: hollow circle (field) or filled circle (method)
            if is_method {
                out.push_str(&format!(
                    "<circle class=\"uml-vis-glyph\" cx=\"{cx}\" cy=\"{gy}\" r=\"4\" fill=\"{color}\" stroke=\"{color}\" stroke-width=\"1\"/>",
                    cx = gx + 4,
                ));
            } else {
                out.push_str(&format!(
                    "<circle class=\"uml-vis-glyph\" cx=\"{cx}\" cy=\"{gy}\" r=\"4\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\"/>",
                    cx = gx + 4,
                ));
            }
        }
        "-" => {
            // Private: filled diamond (same for field and method)
            let cx = gx + 4;
            out.push_str(&format!(
                "<polygon class=\"uml-vis-glyph\" points=\"{cx},{y0} {x1},{gy} {cx},{y2} {x2},{gy}\" fill=\"{color}\" stroke=\"{color}\" stroke-width=\"0.5\"/>",
                y0 = gy - 5,
                x1 = cx + 4,
                y2 = gy + 5,
                x2 = cx - 4,
            ));
        }
        "#" => {
            // Protected: hollow diamond
            let cx = gx + 4;
            out.push_str(&format!(
                "<polygon class=\"uml-vis-glyph\" points=\"{cx},{y0} {x1},{gy} {cx},{y2} {x2},{gy}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\"/>",
                y0 = gy - 5,
                x1 = cx + 4,
                y2 = gy + 5,
                x2 = cx - 4,
            ));
        }
        "~" => {
            // Package: filled triangle pointing up
            let cx = gx + 4;
            out.push_str(&format!(
                "<polygon class=\"uml-vis-glyph\" points=\"{cx},{y0} {x1},{y2} {x2},{y2}\" fill=\"{color}\" stroke=\"{color}\" stroke-width=\"0.5\"/>",
                y0 = gy - 5,
                x1 = cx + 5,
                y2 = gy + 4,
                x2 = cx - 5,
            ));
        }
        _ => {}
    }
    // Glyphs are ~10px wide; shift text right by 14px.
    14
}

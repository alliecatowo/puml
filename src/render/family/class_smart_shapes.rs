//! Smart-default shape renderers for DDD / architectural stereotypes (#1285).
//!
//! Each function renders a specialised SVG shape for a given stereotype and
//! returns immediately.  The dispatch entry-point `render_smart_default_shape`
//! is called from `class_node_render.rs`; it returns `true` when a specialised
//! shape was rendered (caller must `return`) and `false` otherwise.
//! `ddd_smart_header_color` returns the DDD default header fill for the
//! given builtin_type_marker, or `None` if not a DDD stereotype.

#[path = "class_smart_shapes_impl.rs"]
mod impl_;

use impl_::{
    render_corner_u_rect_node, render_cylinder_node, render_double_border_rect_node,
    render_hexagon_node, render_pill_node, render_thick_rounded_rect_node,
};

use super::class_layout::class_node_display_name;
use super::class_members::{count_header_stereotype_members, is_user_stereotype};
use super::class_types::ClassNodeGeometry;

/// Return the smart-default header fill colour for a DDD/arch stereotype.
/// Returns `None` when `builtin_type_marker` is not a DDD stereotype.
pub(super) fn ddd_smart_header_color(
    builtin_type_marker: Option<&'static str>,
) -> Option<&'static str> {
    match builtin_type_marker {
        Some("\u{ab}controller\u{bb}") => Some("#bfdbfe"),
        Some("\u{ab}service\u{bb}") => Some("#bbf7d0"),
        Some("\u{ab}repository\u{bb}") => Some("#fef3c7"),
        Some("\u{ab}value\u{bb}") => Some("#e9d5ff"),
        Some("\u{ab}aggregate\u{bb}") => Some("#ffffff"),
        Some("\u{ab}factory\u{bb}") => Some("#fed7aa"),
        Some("\u{ab}datatype\u{bb}") => Some("#f1f5f9"),
        Some("\u{ab}utility\u{bb}") => Some("#cbd5e1"),
        _ => None,
    }
}

/// Render a DDD / architectural stereotype node using its canonical shape.
///
/// Returns `true` when a specialised shape was rendered (caller must `return`).
/// Returns `false` for stereotypes that have only a header colour but no bespoke
/// geometry, allowing the caller to fall through to the standard rect renderer.
///
/// Shape mapping (const table — issue #1285):
/// | Stereotype      | Shape                          | Header colour |
/// |-----------------|--------------------------------|---------------|
/// | `<<controller>>`| hexagon (flat top/bottom)      | `#bfdbfe`     |
/// | `<<service>>`   | pill / rounded-rect tall       | `#bbf7d0`     |
/// | `<<repository>>`| cylinder                       | `#fef3c7`     |
/// | `<<value>>`     | hexagon (flat top/bottom)      | `#e9d5ff`     |
/// | `<<aggregate>>` | thick-border rounded rect      | `#ffffff`     |
/// | `<<factory>>`   | rounded rect + header band     | `#fed7aa`     |
/// | `<<datatype>>`  | double-border rectangle        | `#f1f5f9`     |
/// | `<<utility>>`   | rectangle + corner U mark      | `#cbd5e1`     |
#[allow(clippy::too_many_arguments)]
pub(super) fn render_smart_default_shape(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    builtin_type_marker: Option<&'static str>,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    namespace_separator: Option<&str>,
    hide_stereotype: bool,
) -> bool {
    let ClassNodeGeometry { x, y, w, h, .. } = geometry;
    let node_id = node.alias.as_deref().unwrap_or(&node.name);
    let display_name = class_node_display_name(node, namespace_separator);
    // Collect any additional user-defined stereotypes beyond the first built-in one,
    // so they still appear in the header even when a smart-default shape is rendered.
    let extra_user_labels: Vec<String> = if hide_stereotype {
        Vec::new()
    } else {
        let header_skip = count_header_stereotype_members(&node.members);
        // Skip the first member (the built-in type marker), collect any remaining
        // leading user stereotypes.
        node.members[..header_skip]
            .iter()
            .skip(1) // skip the primary builtin marker
            .filter_map(|m| {
                if is_user_stereotype(&m.text) {
                    let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
                    Some(format!("\u{ab}{inner}\u{bb}"))
                } else {
                    None
                }
            })
            .collect()
    };

    match builtin_type_marker {
        Some("\u{ab}controller\u{bb}") => {
            render_hexagon_node(
                out,
                node_id,
                &display_name,
                "\u{ab}controller\u{bb}",
                &extra_user_labels,
                "uml-stereotype-controller",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}service\u{bb}") => {
            render_pill_node(
                out,
                node_id,
                &display_name,
                "\u{ab}service\u{bb}",
                &extra_user_labels,
                "uml-stereotype-service",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}repository\u{bb}") => {
            render_cylinder_node(
                out,
                node_id,
                &display_name,
                "\u{ab}repository\u{bb}",
                &extra_user_labels,
                "uml-stereotype-repository",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}value\u{bb}") => {
            render_hexagon_node(
                out,
                node_id,
                &display_name,
                "\u{ab}value\u{bb}",
                &extra_user_labels,
                "uml-stereotype-value",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}aggregate\u{bb}") => {
            render_thick_rounded_rect_node(
                out,
                node_id,
                &display_name,
                "\u{ab}aggregate\u{bb}",
                &extra_user_labels,
                "uml-stereotype-aggregate",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}factory\u{bb}") => {
            // Factory uses the standard rect layout; we only need a distinctive
            // header band.  Fall through to the default renderer but signal
            // "not dispatched" so the caller handles it via the standard path.
            // The header_fill is already set to the salmon colour.
            false
        }
        Some("\u{ab}datatype\u{bb}") => {
            render_double_border_rect_node(
                out,
                node_id,
                &display_name,
                "\u{ab}datatype\u{bb}",
                &extra_user_labels,
                "uml-stereotype-datatype",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}utility\u{bb}") => {
            render_corner_u_rect_node(
                out,
                node_id,
                &display_name,
                "\u{ab}utility\u{bb}",
                &extra_user_labels,
                "uml-stereotype-utility",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        _ => false,
    }
}

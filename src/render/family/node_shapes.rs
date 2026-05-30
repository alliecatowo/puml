use super::class_members::family_node_label;
use super::family_node_shapes::{render_family_node_shape, render_node_stereotype_rows};
use super::tree::render_centered_multiline_text;
use crate::model::{FamilyNode, FamilyNodeKind};
use crate::render::svg::escape_text;
use crate::theme::{effective_component_node_style, ComponentStyle, ComponentStyleMode};

#[derive(Clone, Copy)]
pub(super) struct DeploymentShapeBounds {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

fn render_deployment_stick_shape(
    out: &mut String,
    kind_label: &str,
    bounds: DeploymentShapeBounds,
    fill: &str,
    stroke: &str,
) {
    let DeploymentShapeBounds { x, y, w, h } = bounds;
    let cx = x + w / 2;
    let head_y = y + 16;
    out.push_str(&format!(
        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" fill-opacity=\"0.16\"/>",
        kind_label, x, y, w, h, escape_text(fill), stroke
    ));
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"9\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx,
        head_y,
        escape_text(fill),
        stroke
    ));
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{}\" x2=\"{cx}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>",
        head_y + 9,
        y + h - 24,
        stroke
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>",
        cx - 16,
        y + 36,
        cx + 16,
        y + 36,
        stroke
    ));
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>",
        y + h - 24,
        cx - 12,
        y + h - 8,
        stroke
    ));
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.8\"/>",
        y + h - 24,
        cx + 12,
        y + h - 8,
        stroke
    ));
}

fn render_deployment_queue_shape(
    out: &mut String,
    kind_label: &str,
    bounds: DeploymentShapeBounds,
    fill: &str,
    stroke: &str,
) {
    let DeploymentShapeBounds { x, y, w, h } = bounds;
    let cap = 12;
    let cx_right = x + w - cap;
    let cy = y + h / 2;
    out.push_str(&format!(
        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
        kind_label,
        x + cap,
        y,
        w - cap * 2,
        h,
        escape_text(fill)
    ));
    out.push_str(&format!(
        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{} {} A{} {} 0 0 0 {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        kind_label,
        x + cap,
        y,
        cap,
        h / 2,
        x + cap,
        y + h,
        stroke
    ));
    out.push_str(&format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx_right,
        cy,
        cap,
        h / 2,
        escape_text(fill),
        stroke
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x + cap,
        y,
        cx_right,
        y,
        stroke,
        x + cap,
        y + h,
        cx_right,
        y + h,
        stroke
    ));
}

/// Styled variant of `render_family_node_shape` that applies `comp_style` for
/// Component/Interface nodes and falls back to the default for others.
pub(super) fn render_family_node_shape_styled(
    out: &mut String,
    node: &FamilyNode,
    bounds: DeploymentShapeBounds,
    comp_style: &ComponentStyle,
    hide_stereotype: bool,
) {
    let DeploymentShapeBounds { x, y, w, h } = bounds;
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);
    let effective_style = effective_component_node_style(comp_style, node);
    let stroke = effective_style.stroke.as_str();
    let fill = effective_style.fill.as_str();
    let font_color = effective_style.font_color.as_str();
    let stroke_dash = if effective_style.border_dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    };
    let stroke_width = effective_style.stroke_width;
    out.push_str(&format!(
        "<desc data-uml-id=\"{}\">{}</desc>",
        escape_text(&node.name),
        escape_text(&node.name)
    ));

    match node.kind {
        FamilyNodeKind::Interface => {
            let r = 18;
            out.push_str(&format!(
                "<circle class=\"uml-node uml-interface\" data-uml-kind=\"interface\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                cx, cy, r, fill, escape_text(stroke), stroke_width, stroke_dash
            ));
        }
        FamilyNodeKind::Port => {
            let pw = 24;
            let ph = 24;
            let port_dir = if node.members.iter().any(|m| m.text == "<<portin>>") {
                "in"
            } else if node.members.iter().any(|m| m.text == "<<portout>>") {
                "out"
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"uml-node uml-port\" data-uml-kind=\"port\" data-uml-port-direction=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                escape_text(port_dir),
                cx - pw / 2,
                cy - ph / 2,
                pw,
                ph,
                fill,
                escape_text(stroke),
                stroke_width,
                stroke_dash
            ));
        }
        FamilyNodeKind::Component => {
            // `skinparam roundcorner <N>` overrides the historical 4px radius.
            let rx = comp_style.round_corner.unwrap_or(4);
            // `skinparam shadowing true` drops a soft shadow under the component
            // rect via the `#shadow` filter emitted in the parent SVG defs.
            let shadow_attr = if comp_style.shadowing {
                " filter=\"url(#shadow)\""
            } else {
                ""
            };
            match comp_style.component_style_mode {
                ComponentStyleMode::Rectangle => {
                    // Rectangle style: plain rect, no component icon
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-component\" data-uml-kind=\"component\" data-component-style=\"rectangle\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{shadow_attr}/>",
                        x, y, w, h, fill, escape_text(stroke), stroke_width, stroke_dash
                    ));
                }
                ComponentStyleMode::Uml1 => {
                    // UML1: rectangle with component icon badges in the top-right corner
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-component\" data-uml-kind=\"component\" data-component-style=\"uml1\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{shadow_attr}/>",
                        x, y, w, h, fill, escape_text(stroke), stroke_width, stroke_dash
                    ));
                    let bx = x + w - 18;
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        bx, y + 8, fill, escape_text(stroke)
                    ));
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        bx, y + 20, fill, escape_text(stroke)
                    ));
                }
                ComponentStyleMode::Uml2 => {
                    // UML2 (default): rectangle with badge rects on the left edge
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-component\" data-uml-kind=\"component\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{shadow_attr}/>",
                        x, y, w, h, fill, escape_text(stroke), stroke_width, stroke_dash
                    ));
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x - 4, y + 12, fill, escape_text(stroke)
                    ));
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x - 4, y + h - 20, fill, escape_text(stroke)
                    ));
                }
            }
        }
        FamilyNodeKind::Action
        | FamilyNodeKind::Agent
        | FamilyNodeKind::Node
        | FamilyNodeKind::Frame
        | FamilyNodeKind::Artifact
        | FamilyNodeKind::Boundary
        | FamilyNodeKind::Cloud
        | FamilyNodeKind::Circle
        | FamilyNodeKind::Collections
        | FamilyNodeKind::Storage
        | FamilyNodeKind::Container
        | FamilyNodeKind::Control
        | FamilyNodeKind::Database
        | FamilyNodeKind::Entity
        | FamilyNodeKind::Package
        | FamilyNodeKind::Rectangle
        | FamilyNodeKind::Folder
        | FamilyNodeKind::File
        | FamilyNodeKind::Card
        | FamilyNodeKind::Actor
        | FamilyNodeKind::Hexagon
        | FamilyNodeKind::Label
        | FamilyNodeKind::Person
        | FamilyNodeKind::Process
        | FamilyNodeKind::Queue
        | FamilyNodeKind::Stack
        | FamilyNodeKind::UseCaseDeployment => {
            match node.kind {
                // 3D cube for deployment nodes (fix #571)
                FamilyNodeKind::Node | FamilyNodeKind::Frame => {
                    let offset = 12i32; // 3D depth offset (right and up)
                                        // Top face: parallelogram from front-top edge to back-top edge (shifted right+up).
                                        // Points: front-top-left -> back-top-left -> back-top-right -> front-top-right
                    out.push_str(&format!(
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"#d4dff7\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x,
                        y, // front-top-left
                        x + offset,
                        y - offset, // back-top-left (up + right)
                        x + w + offset,
                        y - offset, // back-top-right
                        x + w,
                        y, // front-top-right
                        stroke
                    ));
                    // Right face: parallelogram from front-right edge to back-right edge.
                    // Points: front-top-right -> back-top-right -> back-bottom-right -> front-bottom-right
                    out.push_str(&format!(
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"#b8c8ef\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x + w,
                        y, // front-top-right
                        x + w + offset,
                        y - offset, // back-top-right
                        x + w + offset,
                        y + h - offset, // back-bottom-right
                        x + w,
                        y + h, // front-bottom-right
                        stroke
                    ));
                    // Front face (main visible face, drawn last so it sits on top)
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" \
                         x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        x,
                        y,
                        w,
                        h,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Database | FamilyNodeKind::Storage => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{x},{top} C{x},{top_minus} {right},{top_minus} {right},{top} L{right},{bottom} C{right},{bottom_plus} {x},{bottom_plus} {x},{bottom} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        escape_text(fill),
                        stroke,
                        top = y + 10,
                        top_minus = y,
                        right = x + w,
                        bottom = y + h - 10,
                        bottom_plus = y + h
                    ));
                    out.push_str(&format!(
                        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx,
                        y + 10,
                        w / 2,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Cloud => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"cloud\" d=\"M{} {} C{} {}, {} {}, {} {} C{} {}, {} {}, {} {} L{} {} C{} {}, {} {}, {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 24, y + 56,
                        x + 4, y + 54, x + 4, y + 28, x + 30, y + 28,
                        x + 36, y + 8, x + 76, y + 8, x + 88, y + 26,
                        x + w - 22, y + 26,
                        x + w - 2, y + 28, x + w - 4, y + 56, x + w - 28, y + 56,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Folder => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"folder\" d=\"M{x},{y} H{} L{} {} H{} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 66,
                        x + 82,
                        y + 14,
                        x + w,
                        y + h,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Artifact | FamilyNodeKind::File => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        x + w - 18,
                        x + w,
                        y + 18,
                        y + h,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Queue => {
                    let bounds = DeploymentShapeBounds { x, y, w, h };
                    render_deployment_queue_shape(out, kind_label, bounds, fill, stroke);
                }
                FamilyNodeKind::Stack | FamilyNodeKind::Collections => {
                    for offset in [10, 5, 0] {
                        out.push_str(&format!(
                            "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                            kind_label,
                            x + offset,
                            y + offset,
                            w - 10,
                            h - 10,
                            escape_text(fill),
                            stroke
                        ));
                    }
                }
                FamilyNodeKind::Hexagon => {
                    out.push_str(&format!(
                        "<polygon class=\"uml-node uml-deployment-shape\" data-uml-kind=\"hexagon\" points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 18,
                        y,
                        x + w - 18,
                        y,
                        x + w,
                        y + h / 2,
                        x + w - 18,
                        y + h,
                        x + 18,
                        y + h,
                        x,
                        y + h / 2,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Circle => {
                    out.push_str(&format!(
                        "<ellipse class=\"uml-node uml-deployment-shape\" data-uml-kind=\"circle\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx,
                        cy,
                        w / 2,
                        h / 2,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::UseCaseDeployment => {
                    out.push_str(&format!(
                        "<ellipse class=\"uml-node uml-deployment-shape\" data-uml-kind=\"usecase\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx,
                        cy,
                        w / 2,
                        h / 2,
                        escape_text(fill),
                        stroke
                    ));
                }
                FamilyNodeKind::Actor | FamilyNodeKind::Person => {
                    let bounds = DeploymentShapeBounds { x, y, w, h };
                    render_deployment_stick_shape(out, kind_label, bounds, fill, stroke);
                }
                FamilyNodeKind::Boundary | FamilyNodeKind::Control | FamilyNodeKind::Entity => {
                    out.push_str(&format!(
                        "<ellipse class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        cx,
                        cy - 4,
                        (w / 2).saturating_sub(12),
                        (h / 2).saturating_sub(12),
                        escape_text(fill),
                        stroke
                    ));
                    if matches!(node.kind, FamilyNodeKind::Boundary) {
                        out.push_str(&format!(
                            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                            x + 12,
                            y + h - 14,
                            x + w - 12,
                            y + h - 14,
                            stroke
                        ));
                    } else if matches!(node.kind, FamilyNodeKind::Control) {
                        out.push_str(&format!(
                            "<path d=\"M{} {} L{} {} L{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                            cx + 6,
                            cy - 14,
                            cx + 24,
                            cy - 22,
                            cx + 18,
                            cy - 4,
                            stroke
                        ));
                    } else {
                        out.push_str(&format!(
                            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                            x + 22,
                            y + h - 18,
                            x + w - 22,
                            y + h - 18,
                            stroke
                        ));
                    }
                }
                _ => {
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label, x, y, w, h, fill, stroke
                    ));
                }
            }
        }
        _ => {
            // Delegate to the non-styled version for all other shapes
            render_family_node_shape(out, node, x, y, w, h, hide_stereotype);
            return;
        }
    }

    // Label
    let (label_x, label_y) = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => (cx, cy + 28),
        _ => (cx, cy + 6),
    };
    let label_last_y = render_centered_multiline_text(
        out,
        label_x,
        label_y,
        13,
        "600",
        Some(font_color),
        &display,
    );
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => label_last_y + 14,
        _ => y + 14,
    };
    // PlantUML parity (#1347): suppress the implicit kind-tag caption
    // ("node", "database", "artifact", «component», «interface», …) entirely.
    // The shape itself is sufficient signal of the kind, and PlantUML never
    // emits these tags. User-supplied `<<stereotype>>` members are still
    // rendered below via `render_node_stereotype_rows`. The previous fixes
    // #525 (Component → «component») and #549 (suppress on Package/Rectangle/
    // Folder containers and `componentStyle rectangle` components) are
    // subsumed by this blanket suppression.
    if !hide_stereotype {
        render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
    }
}

#![allow(dead_code)]

use roxmltree::{Document, Node};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn right(self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(self) -> f64 {
        self.y + self.height
    }
}

pub struct SvgDoc<'a> {
    doc: Document<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvgHookNode {
    pub id: String,
    pub kind: Option<String>,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgSegment {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SvgHookEdge {
    pub from: String,
    pub to: String,
    pub segments: Vec<SvgSegment>,
}

impl<'a> SvgDoc<'a> {
    pub fn parse(svg: &'a str) -> Self {
        let doc = Document::parse(svg).expect("rendered SVG should parse as XML");
        assert_eq!(
            doc.root_element().tag_name().name(),
            "svg",
            "rendered document should have an <svg> root"
        );
        Self { doc }
    }

    pub fn root_attr(&self, name: &str) -> Option<&str> {
        self.doc.root_element().attribute(name)
    }

    pub fn elements(&self, tag: &str) -> Vec<Node<'_, '_>> {
        self.doc
            .descendants()
            .filter(|node| node.is_element() && node.tag_name().name() == tag)
            .collect()
    }

    pub fn elements_with_class(&self, tag: &str, class_name: &str) -> Vec<Node<'_, '_>> {
        self.elements(tag)
            .into_iter()
            .filter(|node| has_class(*node, class_name))
            .collect()
    }

    pub fn elements_with_attr(&self, tag: &str, name: &str, value: &str) -> Vec<Node<'_, '_>> {
        self.elements(tag)
            .into_iter()
            .filter(|node| node.attribute(name) == Some(value))
            .collect()
    }

    pub fn first_with_attr(&self, tag: &str, name: &str, value: &str) -> Node<'_, '_> {
        self.elements_with_attr(tag, name, value)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("expected <{tag}> with {name}={value:?}"))
    }

    pub fn text(&self, expected: &str) -> Node<'_, '_> {
        self.elements("text")
            .into_iter()
            .find(|node| text_content(*node) == expected)
            .unwrap_or_else(|| panic!("expected visible <text> node {expected:?}"))
    }

    pub fn texts_containing(&self, needle: &str) -> Vec<Node<'_, '_>> {
        self.elements("text")
            .into_iter()
            .filter(|node| text_content(*node).contains(needle))
            .collect()
    }

    pub fn hook_nodes(&self) -> Vec<SvgHookNode> {
        self.doc
            .descendants()
            .filter(|node| {
                node.is_element() && (has_class(*node, "puml-node") || has_class(*node, "uml-node"))
            })
            .map(|node| SvgHookNode {
                id: node
                    .attribute("data-puml-id")
                    .or_else(|| node.attribute("data-uml-id"))
                    .or_else(|| node.attribute("id"))
                    .unwrap_or("")
                    .to_string(),
                kind: node
                    .attribute("data-puml-kind")
                    .or_else(|| node.attribute("data-uml-kind"))
                    .map(str::to_string),
                bounds: semantic_bounds(node),
            })
            .collect()
    }

    pub fn hook_edges(&self) -> Vec<SvgHookEdge> {
        self.doc
            .descendants()
            .filter(|node| {
                node.is_element()
                    && (has_class(*node, "puml-edge") || has_class(*node, "uml-relation"))
            })
            .filter_map(|node| {
                let from = node
                    .attribute("data-puml-from")
                    .or_else(|| node.attribute("data-uml-from"))?
                    .to_string();
                let to = node
                    .attribute("data-puml-to")
                    .or_else(|| node.attribute("data-uml-to"))?
                    .to_string();
                Some(SvgHookEdge {
                    from,
                    to,
                    segments: edge_segments(node),
                })
            })
            .collect()
    }
}

pub fn attr<'a>(node: Node<'a, 'a>, name: &str) -> &'a str {
    node.attribute(name)
        .unwrap_or_else(|| panic!("expected <{}> to have {name:?}", node.tag_name().name()))
}

pub fn f64_attr(node: Node<'_, '_>, name: &str) -> f64 {
    attr(node, name)
        .parse::<f64>()
        .unwrap_or_else(|err| panic!("expected numeric SVG attr {name:?}: {err}"))
}

pub fn bounds(node: Node<'_, '_>) -> Bounds {
    match node.tag_name().name() {
        "rect" => Bounds {
            x: f64_attr(node, "x"),
            y: f64_attr(node, "y"),
            width: f64_attr(node, "width"),
            height: f64_attr(node, "height"),
        },
        "circle" => {
            let cx = f64_attr(node, "cx");
            let cy = f64_attr(node, "cy");
            let r = f64_attr(node, "r");
            Bounds {
                x: cx - r,
                y: cy - r,
                width: r * 2.0,
                height: r * 2.0,
            }
        }
        "ellipse" => {
            let cx = f64_attr(node, "cx");
            let cy = f64_attr(node, "cy");
            let rx = f64_attr(node, "rx");
            let ry = f64_attr(node, "ry");
            Bounds {
                x: cx - rx,
                y: cy - ry,
                width: rx * 2.0,
                height: ry * 2.0,
            }
        }
        "line" => {
            let x1 = f64_attr(node, "x1");
            let y1 = f64_attr(node, "y1");
            let x2 = f64_attr(node, "x2");
            let y2 = f64_attr(node, "y2");
            Bounds {
                x: x1.min(x2),
                y: y1.min(y2),
                width: (x1 - x2).abs(),
                height: (y1 - y2).abs(),
            }
        }
        "polygon" | "polyline" => bounds_from_points(attr(node, "points")),
        "text" => Bounds {
            x: f64_attr(node, "x"),
            y: f64_attr(node, "y"),
            width: 0.0,
            height: 0.0,
        },
        tag => panic!("bounds unsupported for <{tag}>"),
    }
}

pub fn semantic_bounds(node: Node<'_, '_>) -> Bounds {
    if let Some(raw) = node.attribute("data-puml-bbox") {
        let parts = raw
            .split_whitespace()
            .map(|part| {
                part.parse::<f64>()
                    .unwrap_or_else(|err| panic!("expected data-puml-bbox number: {err}"))
            })
            .collect::<Vec<_>>();
        if parts.len() == 4 {
            return Bounds {
                x: parts[0],
                y: parts[1],
                width: parts[2],
                height: parts[3],
            };
        }
    }
    bounds(node)
}

pub fn edge_segments(node: Node<'_, '_>) -> Vec<SvgSegment> {
    match node.tag_name().name() {
        "line" => vec![SvgSegment {
            x1: f64_attr(node, "x1"),
            y1: f64_attr(node, "y1"),
            x2: f64_attr(node, "x2"),
            y2: f64_attr(node, "y2"),
        }],
        "polyline" => {
            let points = attr(node, "points")
                .split_whitespace()
                .filter_map(|point| {
                    let (x, y) = point.split_once(',')?;
                    Some((
                        x.parse::<f64>().expect("point x should be numeric"),
                        y.parse::<f64>().expect("point y should be numeric"),
                    ))
                })
                .collect::<Vec<_>>();
            points
                .windows(2)
                .map(|pair| SvgSegment {
                    x1: pair[0].0,
                    y1: pair[0].1,
                    x2: pair[1].0,
                    y2: pair[1].1,
                })
                .collect()
        }
        tag => panic!("edge segments unsupported for <{tag}>"),
    }
}

pub fn text_content(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter_map(|descendant| descendant.text())
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn has_class(node: Node<'_, '_>, class_name: &str) -> bool {
    node.attribute("class")
        .map(|class| class.split_whitespace().any(|part| part == class_name))
        .unwrap_or(false)
}

fn bounds_from_points(points: &str) -> Bounds {
    let mut xs = Vec::new();
    let mut ys = Vec::new();

    for point in points.split_whitespace() {
        let Some((x, y)) = point.split_once(',') else {
            continue;
        };
        xs.push(x.parse::<f64>().expect("point x should be numeric"));
        ys.push(y.parse::<f64>().expect("point y should be numeric"));
    }

    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(min_x.is_finite(), "expected at least one SVG point");

    Bounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

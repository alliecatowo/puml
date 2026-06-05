//! Regression tests for channel router namespace-subframe obstacle avoidance (#1472).
//!
//! The K8s fixture has edges between nodes in DIFFERENT namespaces (e.g.
//! QC in Namespace:backend → PG0 in Namespace:data; ENV in backend → API in
//! backend).  Before the fix, the router treated enclosing cluster frames as
//! opaque obstacles and computed a detour x that escaped the cluster boundary
//! (x ≈ 937 on a 915-wide diagram), producing a massive loop outside the
//! cluster and drawing horizontal lines through sibling node labels.
//!
//! These tests assert that:
//! 1. No edge polyline has a waypoint whose x-coordinate exceeds the cluster
//!    bounding-box right edge (i.e. no path escapes the diagram boundary).
//! 2. No edge polyline segment's horizontal portion passes through a leaf node's
//!    bounding box interior (no lines through labels).

use puml::render_source_to_svg;

const K8S_FIXTURE: &str = r#"@startuml
title Kubernetes pod and container groupings
node "Kubernetes Cluster" {
  node "Namespace: frontend" {
    node "Pod: nginx-proxy" {
      node "nginx" as NGX <<container>>
      node "sidecar-logger" as SL <<container>>
    }
    node "Pod: react-app" {
      node "react-build" as RB <<container>>
    }
  }
  node "Namespace: backend" {
    node "Pod: api-server" {
      node "api-service" as API <<container>>
      node "envoy-proxy" as ENV <<container>>
    }
    node "Pod: worker" {
      node "queue-consumer" as QC <<container>>
    }
  }
  node "Namespace: data" {
    node "StatefulSet: postgres" {
      node "postgres-0" as PG0 <<container>>
      node "postgres-1" as PG1 <<container>>
    }
    node "Pod: redis" {
      node "redis-server" as RS <<container>>
    }
  }
  node "Ingress Controller" as IC
}

IC --> NGX
NGX --> API : HTTP
NGX --> RB
API --> PG0 : psql
API --> RS : redis
QC --> PG0
SL --> IC : log forward
ENV --> API : mTLS
@enduml"#;

/// Parse all polyline `points="…"` values from an SVG string, returning a
/// flat list of (x, y) coordinate pairs per edge.
fn parse_polyline_points(svg: &str) -> Vec<Vec<(f64, f64)>> {
    let mut result = Vec::new();
    let mut search = svg;
    while let Some(start) = search.find("class=\"uml-relation\"") {
        let after = &search[start..];
        if let Some(pts_start) = after.find("points=\"") {
            let pts_inner = &after[pts_start + 8..];
            if let Some(pts_end) = pts_inner.find('"') {
                let pts_str = &pts_inner[..pts_end];
                let points: Vec<(f64, f64)> = pts_str
                    .split_whitespace()
                    .filter_map(|pair| {
                        let mut it = pair.splitn(2, ',');
                        let x: f64 = it.next()?.parse().ok()?;
                        let y: f64 = it.next()?.parse().ok()?;
                        Some((x, y))
                    })
                    .collect();
                if !points.is_empty() {
                    result.push(points);
                }
            }
        }
        // Advance past this relation to find the next one.
        search = &search[start + "class=\"uml-relation\"".len()..];
    }
    result
}

/// Parse the outer cluster frame width from the SVG.
/// Looks for the first `uml-group-frame` rect and returns x + width as the
/// right-edge bound.
fn cluster_right_edge(svg: &str) -> f64 {
    if let Some(pos) = svg.find("class=\"uml-group-frame\"") {
        let after = &svg[pos..];
        // Extract x and width attributes.
        let x: f64 = extract_attr(after, "x=").unwrap_or(0.0);
        let w: f64 = extract_attr(after, "width=").unwrap_or(1024.0);
        return x + w;
    }
    1024.0
}

fn extract_attr(s: &str, attr: &str) -> Option<f64> {
    let pos = s.find(attr)?;
    let after = &s[pos + attr.len()..];
    let after = after.trim_start_matches('"');
    let end = after.find('"').unwrap_or(after.len());
    after[..end].parse().ok()
}

#[test]
fn no_edge_escapes_cluster_boundary() {
    let svg = render_source_to_svg(K8S_FIXTURE).expect("K8s fixture must render without errors");

    let right = cluster_right_edge(&svg);
    // Allow a 4-pixel margin for arrowhead tips that may extend slightly past
    // the port.
    let limit = right + 4.0;

    let all_points = parse_polyline_points(&svg);
    assert!(
        !all_points.is_empty(),
        "Expected polyline edges in SVG but found none"
    );

    for (edge_idx, pts) in all_points.iter().enumerate() {
        for &(x, _y) in pts {
            assert!(
                x <= limit,
                "Edge {edge_idx} has waypoint x={x} which exceeds cluster right edge {right} \
                 (limit={limit}). Massive loop regression detected (#1472)."
            );
        }
    }
}

#[test]
fn no_edge_polyline_crosses_node_labels() {
    let svg = render_source_to_svg(K8S_FIXTURE).expect("K8s fixture must render without errors");

    // Extract leaf node bboxes from SVG rect elements with class "uml-node".
    let node_bboxes = parse_node_bboxes(&svg);

    let all_points = parse_polyline_points(&svg);
    for (edge_idx, pts) in all_points.iter().enumerate() {
        for seg_idx in 0..pts.len().saturating_sub(1) {
            let (x1, y1) = pts[seg_idx];
            let (x2, y2) = pts[seg_idx + 1];
            for &(nx, ny, nw, nh) in &node_bboxes {
                if segment_crosses_box(x1, y1, x2, y2, nx, ny, nw, nh) {
                    panic!(
                        "Edge {edge_idx} segment ({x1},{y1})-({x2},{y2}) \
                         crosses node bbox ({nx},{ny},{},{}) — line-through-label \
                         regression (#1472).",
                        nx + nw,
                        ny + nh
                    );
                }
            }
        }
    }
}

fn parse_node_bboxes(svg: &str) -> Vec<(f64, f64, f64, f64)> {
    let mut bboxes = Vec::new();
    let mut search = svg;
    while let Some(start) = search.find("class=\"uml-node") {
        let after = &search[start..];
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            extract_attr(after, "x="),
            extract_attr(after, "y="),
            extract_attr(after, "width="),
            extract_attr(after, "height="),
        ) {
            bboxes.push((x, y, w, h));
        }
        search = &search[start + "class=\"uml-node".len()..];
    }
    bboxes
}

/// Returns true if the segment (x1,y1)-(x2,y2) passes through the INTERIOR
/// of the given node bounding box (strictly inside, not just touching the boundary).
#[allow(clippy::too_many_arguments)] // 8 args are two paired (x,y) endpoints + (x,y,w,h) bbox — a struct adds no clarity
fn segment_crosses_box(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    nx: f64,
    ny: f64,
    nw: f64,
    nh: f64,
) -> bool {
    let eps = 0.5_f64;
    // Horizontal segment
    if (y1 - y2).abs() < eps {
        let y = y1;
        if y > ny + eps && y < ny + nh - eps {
            let seg_min = x1.min(x2);
            let seg_max = x1.max(x2);
            return seg_min < nx + nw - eps && seg_max > nx + eps;
        }
    }
    // Vertical segment
    if (x1 - x2).abs() < eps {
        let x = x1;
        if x > nx + eps && x < nx + nw - eps {
            let seg_min = y1.min(y2);
            let seg_max = y1.max(y2);
            return seg_min < ny + nh - eps && seg_max > ny + eps;
        }
    }
    false
}

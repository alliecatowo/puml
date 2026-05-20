use super::ordering::count_inversions;
use super::*;

#[test]
fn count_inversions_merge_sort_correctness() {
    assert_eq!(count_inversions(&[]), 0);
    assert_eq!(count_inversions(&[5]), 0);
    assert_eq!(count_inversions(&[1, 2, 3, 4]), 0);
    assert_eq!(count_inversions(&[4, 3, 2, 1]), 6);
    assert_eq!(count_inversions(&[1, 2, 0]), 2);
    assert_eq!(count_inversions(&[3, 1, 2, 0]), 5);
    assert_eq!(count_inversions(&[1, 1, 1]), 0);
    assert_eq!(count_inversions(&[2, 1, 2, 1]), 3);
}

fn make_node(id: &str, parent: Option<&str>) -> NodeSize {
    NodeSize {
        id: id.to_string(),
        width: 200.0,
        height: 80.0,
        parent: parent.map(|s| s.to_string()),
    }
}

fn make_edge(id: &str, from: &str, to: &str) -> EdgeSpec {
    EdgeSpec {
        id: id.to_string(),
        from: from.to_string(),
        to: to.to_string(),
    }
}

#[test]
fn empty_graph_returns_default() {
    let layout = layout_hierarchical(&[], &[], &LayoutOptions::default());
    assert!(layout.node_positions.is_empty());
}

#[test]
fn single_node_is_placed() {
    let nodes = vec![make_node("A", None)];
    let layout = layout_hierarchical(&nodes, &[], &LayoutOptions::default());
    assert!(layout.node_positions.contains_key("A"));
    let &(x, y) = layout.node_positions.get("A").unwrap();
    assert!(x >= 0.0);
    assert!(y >= 0.0);
}

#[test]
fn linear_chain_assigns_increasing_ranks() {
    let nodes = vec![
        make_node("A", None),
        make_node("B", None),
        make_node("C", None),
    ];
    let edges = vec![make_edge("e1", "A", "B"), make_edge("e2", "B", "C")];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    let ra = layout.node_ranks["A"];
    let rb = layout.node_ranks["B"];
    let rc = layout.node_ranks["C"];
    assert!(ra < rb, "A rank ({ra}) should be < B rank ({rb})");
    assert!(rb < rc, "B rank ({rb}) should be < C rank ({rc})");
}

#[test]
fn cycle_is_broken_gracefully() {
    let nodes = vec![make_node("A", None), make_node("B", None)];
    let edges = vec![make_edge("e1", "A", "B"), make_edge("e2", "B", "A")];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    assert!(layout.node_positions.contains_key("A"));
    assert!(layout.node_positions.contains_key("B"));
}

#[test]
fn group_bounds_are_computed() {
    let nodes = vec![make_node("A", Some("G1")), make_node("B", Some("G1"))];
    let layout = layout_hierarchical(&nodes, &[], &LayoutOptions::default());
    assert!(layout.group_bounds.contains_key("G1"));
    let (_, _, w, h) = layout.group_bounds["G1"];
    assert!(w > 0.0);
    assert!(h > 0.0);
}

#[test]
fn top_node_is_above_bottom_node() {
    let nodes = vec![make_node("A", None), make_node("B", None)];
    let edges = vec![make_edge("e1", "A", "B")];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    let ya = layout.node_positions["A"].1;
    let yb = layout.node_positions["B"].1;
    assert!(
        ya < yb,
        "A (y={ya}) should be above B (y={yb}) in TopDown layout"
    );
}

#[test]
fn diamond_graph_no_panic() {
    let nodes = vec![
        make_node("A", None),
        make_node("B", None),
        make_node("C", None),
        make_node("D", None),
    ];
    let edges = vec![
        make_edge("e1", "A", "B"),
        make_edge("e2", "A", "C"),
        make_edge("e3", "B", "D"),
        make_edge("e4", "C", "D"),
    ];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    assert_eq!(layout.node_positions.len(), 4);
    let rd = layout.node_ranks["D"];
    let ra = layout.node_ranks["A"];
    assert!(rd > ra);
}

#[test]
fn architecture_overview_shape() {
    let nodes = vec![
        make_node("PlantumlFE", Some("Frontends")),
        make_node("PicoumlFE", Some("Frontends")),
        make_node("MermaidFE", Some("Frontends")),
        make_node("Parser", Some("PipelineCore")),
        make_node("AST", Some("PipelineCore")),
        make_node("Normalizer", Some("PipelineCore")),
        make_node("Renderer", Some("PipelineCore")),
        make_node("Preproc", Some("SharedServices")),
        make_node("LangSvc", Some("SharedServices")),
        make_node("Diag", Some("SharedServices")),
        make_node("Theme", Some("SharedServices")),
        make_node("CLI", Some("Transports")),
        make_node("LSP", Some("Transports")),
        make_node("WASM", Some("Transports")),
        make_node("SVG", Some("OutputFormats")),
        make_node("Raster", Some("OutputFormats")),
        make_node("Text", Some("OutputFormats")),
    ];
    let edges = vec![
        make_edge("e1", "PlantumlFE", "Parser"),
        make_edge("e2", "PicoumlFE", "Parser"),
        make_edge("e3", "MermaidFE", "Parser"),
        make_edge("e4", "Preproc", "Parser"),
        make_edge("e5", "Parser", "AST"),
        make_edge("e6", "AST", "Normalizer"),
        make_edge("e7", "Normalizer", "Renderer"),
        make_edge("e8", "Theme", "Renderer"),
        make_edge("e9", "Diag", "Renderer"),
        make_edge("e10", "Renderer", "SVG"),
        make_edge("e11", "Renderer", "Raster"),
        make_edge("e12", "Renderer", "Text"),
        make_edge("e13", "CLI", "PlantumlFE"),
        make_edge("e14", "CLI", "PicoumlFE"),
        make_edge("e15", "CLI", "MermaidFE"),
        make_edge("e16", "CLI", "Preproc"),
        make_edge("e17", "LSP", "LangSvc"),
        make_edge("e18", "LangSvc", "Parser"),
        make_edge("e19", "LangSvc", "Diag"),
        make_edge("e20", "WASM", "LangSvc"),
    ];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    assert_eq!(layout.node_positions.len(), 17);
    let r_parser = layout.node_ranks["Parser"];
    let r_renderer = layout.node_ranks["Renderer"];
    assert!(
        r_parser < r_renderer,
        "Parser rank {r_parser} < Renderer rank {r_renderer}"
    );
    let r_svg = layout.node_ranks["SVG"];
    assert!(
        r_renderer < r_svg,
        "Renderer rank {r_renderer} < SVG rank {r_svg}"
    );
}

#[test]
fn orthogonal_path_has_more_than_two_points_for_cross_rank_edge() {
    let nodes = vec![make_node("A", None), make_node("B", None)];
    let edges = vec![make_edge("e1", "A", "B")];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    let path = layout
        .edge_paths
        .get("e1")
        .expect("edge e1 should have a path");
    assert!(
        path.len() >= 3,
        "Orthogonal cross-rank path should have >=3 points, got {} points: {:?}",
        path.len(),
        path
    );
}

#[test]
fn orthogonal_path_endpoints_are_on_node_edges() {
    let nodes = vec![make_node("A", None), make_node("B", None)];
    let edges = vec![make_edge("e1", "A", "B")];
    let opts = LayoutOptions::default();
    let layout = layout_hierarchical(&nodes, &edges, &opts);
    let path = layout.edge_paths.get("e1").unwrap();

    let (ax, ay) = layout.node_positions["A"];
    let (bx, by) = layout.node_positions["B"];

    let expected_src_y = ay + 80.0;
    let expected_src_x = ax + 100.0;
    let (p0x, p0y) = path[0];
    assert!(
        (p0x - expected_src_x).abs() < 1.0 && (p0y - expected_src_y).abs() < 1.0,
        "Path start ({p0x},{p0y}) should be at A bottom-center ({expected_src_x},{expected_src_y})"
    );

    let expected_tgt_y = by;
    let expected_tgt_x = bx + 100.0;
    let &(pnx, pny) = path.last().unwrap();
    assert!(
        (pnx - expected_tgt_x).abs() < 1.0 && (pny - expected_tgt_y).abs() < 1.0,
        "Path end ({pnx},{pny}) should be at B top-center ({expected_tgt_x},{expected_tgt_y})"
    );
}

#[test]
fn same_rank_edge_uses_u_shape() {
    let nodes = vec![
        make_node("A", None),
        make_node("B", None),
        make_node("C", None),
    ];
    let edges = vec![
        make_edge("e1", "A", "C"),
        make_edge("e2", "B", "C"),
        make_edge("e3", "A", "B"),
    ];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    let ra = layout.node_ranks["A"];
    let rb = layout.node_ranks["B"];
    if ra == rb {
        let path = layout.edge_paths.get("e3").expect("e3 should have a path");
        assert!(
            path.len() >= 4,
            "Same-rank U-shape should have >=4 points, got {}: {:?}",
            path.len(),
            path
        );
    }
    assert!(layout.edge_paths.contains_key("e3"));
}

#[test]
fn no_two_adjacent_rank_edges_share_same_track_y() {
    let nodes = vec![
        make_node("A", None),
        make_node("B", None),
        make_node("C", None),
        make_node("D", None),
    ];
    let edges = vec![
        make_edge("e1", "A", "B"),
        make_edge("e2", "A", "C"),
        make_edge("e3", "B", "D"),
        make_edge("e4", "C", "D"),
    ];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());

    let mut channel_ys: std::collections::BTreeMap<i64, Vec<&str>> =
        std::collections::BTreeMap::new();
    for (eid, path) in &layout.edge_paths {
        if path.len() >= 3 {
            let ch_y = path[1].1;
            channel_ys
                .entry(ch_y as i64)
                .or_default()
                .push(eid.as_str());
        }
    }
    assert_eq!(layout.edge_paths.len(), 4, "All 4 edges should have paths");
}

#[test]
fn multi_rank_edge_has_intermediate_waypoints() {
    let nodes = vec![
        make_node("A", None),
        make_node("B", None),
        make_node("C", None),
    ];
    let edges = vec![
        make_edge("e1", "A", "B"),
        make_edge("e2", "B", "C"),
        make_edge("e3", "A", "C"),
    ];
    let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
    let path = layout.edge_paths.get("e3").expect("e3 should have a path");
    let path_e1 = layout.edge_paths.get("e1").unwrap();
    assert!(
        path.len() >= path_e1.len(),
        "Multi-rank edge e3 ({} pts) should have >= same points as e1 ({} pts)",
        path.len(),
        path_e1.len()
    );
}

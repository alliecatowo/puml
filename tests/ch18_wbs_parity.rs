use puml::model::{FamilyStyle, NormalizedDocument};

const WBS_THEME_VIBRANT_SRC: &str = r##"@startwbs
!theme vibrant
* Platform Launch
** Build
*** API
*** UI
** Validate
*** QA
*** Rollout
@endwbs
"##;

#[test]
fn wbs_theme_preset_is_carried_as_depth_style() {
    let document = puml::parser::parse(WBS_THEME_VIBRANT_SRC).expect("parse themed wbs");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize themed wbs")
    else {
        panic!("wbs diagram should normalize as Family");
    };

    let Some(FamilyStyle::MindMap(style)) = model.family_style else {
        panic!("wbs theme should be carried via tree depth style");
    };

    assert_eq!(
        style
            .depth_styles
            .get(&0)
            .and_then(|depth| depth.background_color.as_deref()),
        Some("#f5f3ff")
    );
    assert_eq!(
        style
            .depth_styles
            .get(&1)
            .and_then(|depth| depth.background_color.as_deref()),
        Some("#ede9fe")
    );
    assert_eq!(
        style
            .depth_styles
            .get(&2)
            .and_then(|depth| depth.border_color.as_deref()),
        Some("#d97706")
    );
}

/// #1467 — PlantUML layout parity: WBS default top-to-bottom layout uses
/// PlantUML's `Fork` (root) + `ITFComposed` (depth ≥ 1 vertical stack with
/// orthogonal "+-" connectors). Depth-2+ children should sit BELOW their
/// depth-1 parent (greater y) and to the right (≥ parent x). This replaces
/// the previous horizontal-spread leaves convention that produced wide
/// canvases.
#[test]
fn wbs_default_layout_stacks_grandchildren_below_branch() {
    let src = "@startwbs\n\
               * Root\n\
               ** Branch A\n\
               *** Leaf A1\n\
               *** Leaf A2\n\
               *** Leaf A3\n\
               @endwbs\n";
    let svg = puml::render_source_to_svg(src).expect("wbs should render");

    // Extract rect y positions for nodes by their `data-wbs-depth` attribute.
    fn ys_for_depth(svg: &str, depth: i32) -> Vec<i32> {
        let marker = format!("data-wbs-depth=\"{depth}\"");
        let mut ys = Vec::new();
        let mut search = svg;
        while let Some(idx) = search.find(&marker) {
            // backtrack to <rect
            if let Some(rect_start) = search[..idx].rfind("<rect") {
                let tail = &search[rect_start..];
                let end = tail.find("/>").unwrap_or(tail.len());
                let elem = &tail[..end];
                if let Some(pos) = elem.find(" y=\"") {
                    let start = pos + 4;
                    if let Some(end) = elem[start..].find('"') {
                        if let Ok(y) = elem[start..start + end].parse::<i32>() {
                            ys.push(y);
                        }
                    }
                }
            }
            search = &search[idx + 1..];
        }
        ys
    }

    let depth1_ys = ys_for_depth(&svg, 1);
    let depth2_ys = ys_for_depth(&svg, 2);
    assert!(
        !depth1_ys.is_empty() && !depth2_ys.is_empty(),
        "must have depth-1 and depth-2 nodes"
    );

    let branch_y = depth1_ys[0];
    for leaf_y in &depth2_ys {
        assert!(
            *leaf_y > branch_y,
            "depth-2 leaf y={leaf_y} must sit below depth-1 branch y={branch_y} (PlantUML vstack #1467)"
        );
    }

    // Depth-2 children should stack vertically: all y values strictly increasing.
    let mut sorted = depth2_ys.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted, depth2_ys,
        "depth-2 leaves should already be in vertical stack order (top→bottom)"
    );
    for window in sorted.windows(2) {
        assert!(
            window[1] > window[0] + 20,
            "stacked WBS leaves should have a gap, got {} then {}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn wbs_theme_preset_colors_depth_nodes_but_keeps_inline_fill_override() {
    let src = r##"@startwbs
!theme vibrant
* Platform Launch
**[#pink] Build
*** API
@endwbs
"##;
    let svg = puml::render_source_to_svg(src).expect("render themed wbs");

    assert!(
        svg.contains("class=\"wbs-node wbs-depth-0")
            && svg.contains("data-wbs-fill=\"#f5f3ff\"")
            && svg.contains("stroke=\"#8b5cf6\""),
        "root should use vibrant group colors; svg={svg}"
    );
    assert!(
        svg.contains("data-wbs-fill=\"pink\""),
        "inline WBS fill should still override theme fill; svg={svg}"
    );
    assert!(
        svg.contains("data-wbs-fill=\"#fef3c7\"") && svg.contains("stroke=\"#d97706\""),
        "depth-2 leaf should use vibrant note colors; svg={svg}"
    );
}

#!/usr/bin/env python3
"""Apply render-only ch04 patches to a clean origin/main family.rs."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
p = ROOT / "src/render/family.rs"
t = p.read_text()

HELPERS = '''
        FamilyNodeKind::C4Boundary => "boundary",
        FamilyNodeKind::Map => "map",
        FamilyNodeKind::Diamond => "diamond",
    }
}

const MAP_ROW_HEIGHT: i32 = 18;
const MAP_COL_PAD: i32 = 10;

fn map_entry_parts(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("\\x1fmap:")?;
    rest.split_once('\\x1f')
}

fn count_map_display_rows(members: &[crate::ast::ClassMember]) -> usize {
    members.iter().filter(|m| map_entry_parts(&m.text).is_some()).count()
}

fn map_row_index(members: &[crate::ast::ClassMember], row_key: &str) -> Option<usize> {
    members.iter().position(|m| {
        map_entry_parts(&m.text)
            .is_some_and(|(key, _)| key == row_key || key.eq_ignore_ascii_case(row_key))
    })
}

fn map_row_center_y(box_y: i32, header_h: i32, row_idx: usize) -> i32 {
    box_y + header_h + 16 + (row_idx as i32) * MAP_ROW_HEIGHT + MAP_ROW_HEIGHT / 2
}

fn find_family_node<'a>(nodes: &'a [crate::model::FamilyNode], key: &str) -> Option<&'a crate::model::FamilyNode> {
    nodes.iter().find(|n| n.alias.as_deref() == Some(key) || n.name == key)
}

fn apply_map_qualified_anchor(
    full_endpoint: &str,
    nodes: &[crate::model::FamilyNode],
    node_box: ClassNodeBox,
    anchor_x: i32,
    anchor_y: i32,
) -> (i32, i32) {
    let Some((owner, row_key)) = full_endpoint.rsplit_once("::") else {
        return (anchor_x, anchor_y);
    };
    let Some(node) = find_family_node(nodes, owner) else {
        return (anchor_x, anchor_y);
    };
    if node.kind != FamilyNodeKind::Map {
        return (anchor_ax, anchor_y);
    }
    let Some(row_idx) = map_row_index(&node.members, row_key) else {
        return (anchor_x, anchor_y);
    };
    (anchor_x, map_row_center_y(node_box.y, node_box.header_h, row_idx))
}

struct ClassNodeGeometry'''

t = t.replace(
    '        FamilyNodeKind::C4Boundary => "boundary",\n    }\n}\n\nstruct ClassNodeGeometry',
    HELPERS,
    1,
)

t = t.replace(
    """        let (x1, y1, x2, y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };

        let lat_offset = ctx.parallel_offset.get(&rel_idx).copied().unwrap_or(0);""",
    """        let (mut x1, mut y1, mut x2, mut y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };
        (x1, y1) = apply_map_qualified_anchor(&from_name, ctx.nodes, *from, x1, y1);
        (x2, y2) = apply_map_qualified_anchor(&to_name, ctx.nodes, *to, x2, y2);

        let lat_offset = ctx.parallel_offset.get(&rel_idx).copied().unwrap_or(0);""",
    1,
)

# fix typo in script
t = t.replace("return (anchor_ax, anchor_y)", "return (anchor_x, anchor_y)")

p.write_text(t)
print("render helpers+edges ok")

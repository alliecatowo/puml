use super::super::*;

pub(in crate::render::state) fn position_state_notes(
    nodes: &[StateNode],
    transitions: &[StateTransition],
    sizes: &std::collections::BTreeMap<String, (i32, i32)>,
    placed: &mut std::collections::BTreeMap<String, PlacedNode>,
) {
    let note_names: std::collections::BTreeSet<&str> = nodes
        .iter()
        .filter(|node| node.kind == StateNodeKind::Note)
        .map(|node| node.name.as_str())
        .collect();

    for t in transitions {
        if !note_names.contains(t.to.as_str()) {
            continue;
        }
        let (note_w, note_h) = sizes
            .get(&t.to)
            .copied()
            .unwrap_or((STATE_NODE_W, STATE_NODE_H));

        let mut link_segment = None;
        let (position, anchor_x, anchor_y, target_box) = if let Some((position, target)) =
            parse_state_note_on_link_direction(t.direction.as_deref())
        {
            let Some(from_p) = placed.get(&t.from) else {
                continue;
            };
            let Some(to_p) = placed.get(target) else {
                continue;
            };
            let (x1, y1, x2, y2) = edge_anchors(from_p, to_p);
            link_segment = Some((x1, y1, x2, y2));
            (position, (x1 + x2) / 2, (y1 + y2) / 2, None)
        } else {
            let Some(target_p) = placed.get(&t.from) else {
                continue;
            };
            (
                t.direction.as_deref().unwrap_or("right"),
                target_p.x + target_p.w / 2,
                target_p.y + target_p.h / 2,
                Some(*target_p),
            )
        };

        let gap = 28;
        let (x, mut y) = if let Some(target_p) = target_box {
            match position.to_ascii_lowercase().as_str() {
                "left" => (target_p.x - note_w - gap, anchor_y - note_h / 2),
                "top" | "over" => (anchor_x - note_w / 2, target_p.y - note_h - gap),
                "bottom" => (anchor_x - note_w / 2, target_p.y + target_p.h + gap),
                _ => (target_p.x + target_p.w + gap, anchor_y - note_h / 2),
            }
        } else {
            let vertical_link = link_segment
                .map(|(x1, y1, x2, y2)| (y2 - y1).abs() >= (x2 - x1).abs())
                .unwrap_or(false);
            match position.to_ascii_lowercase().as_str() {
                "left" => (anchor_x - note_w - gap, anchor_y - note_h / 2),
                "top" | "over" if vertical_link => (anchor_x + gap, anchor_y - note_h / 2),
                "top" | "over" => (anchor_x - note_w / 2, anchor_y - note_h - gap),
                "bottom" => (anchor_x - note_w / 2, anchor_y + gap),
                _ => (anchor_x + gap, anchor_y - note_h / 2),
            }
        };
        while placed
            .iter()
            .any(|(name, other)| name != &t.to && rects_overlap(x, y, note_w, note_h, other))
        {
            y += note_h + 12;
        }
        placed.insert(
            t.to.clone(),
            PlacedNode {
                x,
                y,
                w: note_w,
                h: note_h,
            },
        );
    }
}

pub(in crate::render::state) fn rects_overlap(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    other: &PlacedNode,
) -> bool {
    x < other.x + other.w && x + w > other.x && y < other.y + other.h && y + h > other.y
}

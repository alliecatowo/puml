use super::*;

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
    // Extract activity style (use defaults if not present)
    let act_style = match &doc.family_style {
        Some(FamilyStyle::Activity(s)) => s.clone(),
        _ => ActivityStyle::default(),
    };

    let step_h = 60i32;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;

    // ---------------------------------------------------------------------------
    // Pass 0: parse node metadata (step_kind, lane_name) for all nodes
    // ---------------------------------------------------------------------------
    struct NodeMeta {
        step_kind: String,
        lane_name: String,
        fork_branch: usize,
    }
    let metas: Vec<NodeMeta> = doc
        .nodes
        .iter()
        .map(|node| {
            let mut step_kind = String::new();
            let mut lane_name = "default".to_string();
            let mut fork_branch = 0usize;
            if let Some(alias) = &node.alias {
                if let Some(meta) = alias.strip_prefix("activity::") {
                    for (pi, part) in meta.split('|').enumerate() {
                        if pi == 0 {
                            step_kind = part.to_string();
                            continue;
                        }
                        if let Some(v) = part.strip_prefix("lane=") {
                            lane_name = v.to_string();
                        } else if let Some(v) = part.strip_prefix("fork_branch=") {
                            fork_branch = v.parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            }
            NodeMeta {
                step_kind,
                lane_name,
                fork_branch,
            }
        })
        .collect();

    // ---------------------------------------------------------------------------
    // Collect swim-lanes
    // ---------------------------------------------------------------------------
    let mut lanes: Vec<String> = Vec::new();
    for meta in &metas {
        if meta.lane_name != "default" && !lanes.iter().any(|l| l == &meta.lane_name) {
            lanes.push(meta.lane_name.clone());
        }
    }
    if lanes.is_empty() {
        lanes.push("default".to_string());
    }

    // Compute the base width from the number of lanes; we may widen for if/else
    // and for fork parallel columns.
    // Count max nesting depth of if/else to estimate extra width needed.
    let mut max_if_depth: i32 = 0;
    {
        let mut depth: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "IfStart" {
                depth += 1;
                max_if_depth = max_if_depth.max(depth);
            } else if meta.step_kind == "EndIf" {
                depth = depth.saturating_sub(1);
            }
        }
    }
    // Count max fork branch count to size canvas for parallel columns.
    let mut max_fork_branches: i32 = 0;
    {
        let mut count: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "Fork" {
                count = 1;
            } else if meta.step_kind == "ForkAgain" {
                count += 1;
                max_fork_branches = max_fork_branches.max(count);
            } else if meta.step_kind == "EndFork" {
                count = 0;
            }
        }
    }
    // Branch horizontal offset: each nesting level adds 160px to either side
    let branch_x_offset = 160i32;
    // Total extra width: 2 * branch_x_offset * max_if_depth (left + right of center)
    let extra_branch_width = 2 * branch_x_offset * max_if_depth;
    // Extra width for fork parallel columns (each additional branch beyond 1 adds 160px)
    let extra_fork_width = (max_fork_branches * 160i32).max(0);

    let lane_area_x = 32i32;
    let base_lane_area_w = 416i32; // 480 - 64
    let lane_area_w = base_lane_area_w + extra_branch_width + extra_fork_width;
    let width = lane_area_w + 64;
    let lane_w = (lane_area_w / (lanes.len() as i32)).max(120);
    let lane_index = |name: &str| -> i32 {
        lanes
            .iter()
            .position(|l| l == name)
            .map(|i| i as i32)
            .unwrap_or(0)
    };
    let lane_center_x = |lane_name: &str| -> i32 {
        let idx = lane_index(lane_name);
        lane_area_x + idx * lane_w + lane_w / 2
    };

    // Width reserved for each fork branch column
    let fork_col_w = (lane_w / 2).max(160i32);

    // ---------------------------------------------------------------------------
    // Pass 1: compute layout positions for every node using a branch-aware
    // algorithm.
    //
    // For each node:
    //   slot_y      - top of the slot (y passed to shape renderers)
    //   arrow_out_y - where the outgoing arrow starts (slot_y + ARROW_OUT)
    //   next_slot_y - where the next node's slot begins (slot_y + step_h)
    //
    // current_slot_y tracks where the next node goes.
    // if_stack handles nested if/else.
    // fork_stack handles parallel fork/join branches.
    // ---------------------------------------------------------------------------

    const ARROW_OUT: i32 = 42; // visual bottom of a node within its slot

    struct IfFrame {
        diamond_cx: i32,
        diamond_arrow_out: i32,
        diamond_next_slot: i32,
        then_cx: i32,
        then_rightmost_cx: i32,
        then_end_next_slot: i32,
        in_else: bool,
        else_cx: i32,
        else_start_slot: i32,
    }

    // Fork frame: tracks parallel branches in a fork/join block.
    struct ForkFrame {
        fork_node_idx: usize,
        fork_cx: i32,
        fork_slot_y: i32,
        branch_start_y: i32,
        branches: Vec<ForkBranch>,
        current_branch: usize,
        fork_again_indices: Vec<usize>,
    }
    struct ForkBranch {
        start_node_idx: usize,
        end_next_slot: i32,
    }

    // Per-node layout
    struct NodeLayout {
        cx: i32,
        slot_y: i32,
        arrow_out_y: i32,
        next_slot_y: i32,
    }

    let mut node_layouts: Vec<NodeLayout> = Vec::with_capacity(doc.nodes.len());
    // Fork bar half-widths: maps node index to half-width of the fork/join bar
    let mut fork_bar_half_widths: std::collections::HashMap<usize, i32> = Default::default();
    // Extra arrows: (x1,y1, x2,y2) drawn in addition to prev->cur arrows
    let mut extra_arrows: Vec<(i32, i32, i32, i32)> = Vec::new();
    // Indices of nodes for which we suppress the standard prev->cur arrow
    let mut suppress_prev_arrow: std::collections::HashSet<usize> = Default::default();

    let mut current_slot_y = header_h;
    let mut if_stack: Vec<IfFrame> = Vec::new();
    let mut fork_stack: Vec<ForkFrame> = Vec::new();

    for (i, meta) in metas.iter().enumerate() {
        let base_cx = lane_center_x(&meta.lane_name);
        let cx = if_stack
            .last()
            .map(|f| if f.in_else { f.else_cx } else { f.then_cx })
            .unwrap_or(base_cx);

        // Inside a fork: use branch column cx. Will be fixed up retroactively in EndFork.
        let cx = if let Some(frame) = fork_stack.last() {
            let n_branches = frame.branches.len();
            let branch_idx = frame.current_branch;
            fork_branch_cx(frame.fork_cx, branch_idx, n_branches, fork_col_w)
        } else {
            cx
        };

        match meta.step_kind.as_str() {
            "IfStart" => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                let else_cx = cx + branch_x_offset;
                if_stack.push(IfFrame {
                    diamond_cx: cx,
                    diamond_arrow_out: arrow_out_y,
                    diamond_next_slot: next_slot_y,
                    then_cx: cx,
                    then_rightmost_cx: cx,
                    then_end_next_slot: next_slot_y,
                    in_else: false,
                    else_cx,
                    else_start_slot: next_slot_y,
                });
                for frame in &mut if_stack {
                    if !frame.in_else {
                        frame.then_rightmost_cx = frame.then_rightmost_cx.max(cx);
                    }
                }
                current_slot_y = next_slot_y;
            }
            "Else" => {
                let then_end_next_slot = current_slot_y;
                let frame = if_stack.last_mut().expect("else without if");
                frame.then_cx = cx;
                frame.then_end_next_slot = then_end_next_slot;
                let else_cx = (frame.diamond_cx + branch_x_offset)
                    .max(frame.then_rightmost_cx + branch_x_offset);
                frame.else_cx = else_cx;
                let diamond_cx = frame.diamond_cx;
                let diamond_arrow_out = frame.diamond_arrow_out;
                let slot_y = frame.diamond_next_slot;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                frame.else_start_slot = slot_y;
                frame.in_else = true;
                for frame in &mut if_stack {
                    if !frame.in_else {
                        frame.then_rightmost_cx = frame.then_rightmost_cx.max(else_cx);
                    }
                }
                suppress_prev_arrow.insert(i);
                extra_arrows.push((diamond_cx, diamond_arrow_out, else_cx, slot_y));
                node_layouts.push(NodeLayout {
                    cx: else_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                current_slot_y = next_slot_y;
            }
            "EndIf" => {
                let frame = if_stack.pop().expect("endif without if");
                let then_arrow_out_y = frame.then_end_next_slot - step_h + ARROW_OUT;
                let then_cx = frame.then_cx;
                let else_arrow_out_y = current_slot_y - step_h + ARROW_OUT;
                let else_cx = frame.else_cx;
                let slot_y = frame.then_end_next_slot.max(current_slot_y);
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                suppress_prev_arrow.insert(i);
                extra_arrows.push((then_cx, then_arrow_out_y, frame.diamond_cx, slot_y));
                extra_arrows.push((else_cx, else_arrow_out_y, frame.diamond_cx, slot_y));
                node_layouts.push(NodeLayout {
                    cx: frame.diamond_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                for parent in &mut if_stack {
                    if !parent.in_else {
                        parent.then_rightmost_cx =
                            parent.then_rightmost_cx.max(frame.then_rightmost_cx);
                    }
                }
                current_slot_y = next_slot_y;
            }
            "Fork" => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                fork_stack.push(ForkFrame {
                    fork_node_idx: i,
                    fork_cx: cx,
                    fork_slot_y: slot_y,
                    branch_start_y: next_slot_y,
                    branches: vec![ForkBranch {
                        start_node_idx: i + 1,
                        end_next_slot: next_slot_y,
                    }],
                    current_branch: 0,
                    fork_again_indices: Vec::new(),
                });
                current_slot_y = next_slot_y;
            }
            "ForkAgain" => {
                let frame = fork_stack.last_mut().expect("fork again without fork");
                let branch_idx = frame.current_branch;
                frame.branches[branch_idx].end_next_slot = current_slot_y;
                frame.fork_again_indices.push(i);
                frame.branches.push(ForkBranch {
                    start_node_idx: i + 1,
                    end_next_slot: frame.branch_start_y,
                });
                frame.current_branch += 1;
                let n_branches = frame.branches.len();
                let new_branch_idx = frame.current_branch;
                let fork_cx = frame.fork_cx;
                suppress_prev_arrow.insert(i);
                let slot_y = frame.fork_slot_y;
                let branch_col_cx =
                    fork_branch_cx(fork_cx, new_branch_idx, n_branches, fork_col_w);
                node_layouts.push(NodeLayout {
                    cx: branch_col_cx,
                    slot_y,
                    arrow_out_y: slot_y + ARROW_OUT,
                    next_slot_y: slot_y + step_h,
                });
                current_slot_y = frame.branch_start_y;
            }
            "EndFork" => {
                let mut frame = fork_stack.pop().expect("endfork without fork");
                let last_branch = frame.current_branch;
                frame.branches[last_branch].end_next_slot = current_slot_y;

                let n_branches = frame.branches.len();
                let fork_cx = frame.fork_cx;
                let branch_start_y = frame.branch_start_y;

                let max_end = frame
                    .branches
                    .iter()
                    .map(|b| b.end_next_slot)
                    .max()
                    .unwrap_or(current_slot_y);
                let slot_y = max_end;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;

                suppress_prev_arrow.insert(i);
                node_layouts.push(NodeLayout {
                    cx: fork_cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });

                // Fix up cx positions for all nodes inside branches
                for (branch_idx, branch) in frame.branches.iter().enumerate() {
                    let col_cx = fork_branch_cx(fork_cx, branch_idx, n_branches, fork_col_w);
                    let branch_end_idx = if branch_idx + 1 < n_branches {
                        frame.fork_again_indices[branch_idx]
                    } else {
                        i
                    };
                    for node_idx in branch.start_node_idx..branch_end_idx {
                        if let Some(layout) = node_layouts.get_mut(node_idx) {
                            layout.cx = col_cx;
                        }
                    }
                    if branch_idx + 1 < n_branches {
                        let fa_idx = frame.fork_again_indices[branch_idx];
                        if let Some(layout) = node_layouts.get_mut(fa_idx) {
                            let next_col_cx = fork_branch_cx(
                                fork_cx,
                                branch_idx + 1,
                                n_branches,
                                fork_col_w,
                            );
                            layout.cx = next_col_cx;
                        }
                    }
                    // Arrow from branch end to join bar
                    let branch_arrow_out_y = branch.end_next_slot - step_h + ARROW_OUT;
                    extra_arrows.push((col_cx, branch_arrow_out_y, fork_cx, slot_y));
                }

                // Arrows from fork bar down into each branch column
                let fork_bar_arrow_out_y = frame.fork_slot_y + ARROW_OUT;
                for branch_idx in 0..n_branches {
                    let col_cx = fork_branch_cx(fork_cx, branch_idx, n_branches, fork_col_w);
                    extra_arrows.push((fork_cx, fork_bar_arrow_out_y, col_cx, branch_start_y));
                }

                // Compute bar half-width spanning all branch columns
                let bar_span_half = if n_branches > 1 {
                    let leftmost =
                        fork_branch_cx(fork_cx, 0, n_branches, fork_col_w) - fork_col_w / 2;
                    let rightmost =
                        fork_branch_cx(fork_cx, n_branches - 1, n_branches, fork_col_w)
                            + fork_col_w / 2;
                    (rightmost - leftmost) / 2
                } else {
                    (lane_w - 24).clamp(60, 110)
                };
                fork_bar_half_widths.insert(frame.fork_node_idx, bar_span_half);
                fork_bar_half_widths.insert(i, bar_span_half);

                current_slot_y = next_slot_y;
            }
            _ => {
                let slot_y = current_slot_y;
                let arrow_out_y = slot_y + ARROW_OUT;
                let next_slot_y = slot_y + step_h;
                node_layouts.push(NodeLayout {
                    cx,
                    slot_y,
                    arrow_out_y,
                    next_slot_y,
                });
                for frame in &mut if_stack {
                    if !frame.in_else {
                        frame.then_rightmost_cx = frame.then_rightmost_cx.max(cx);
                    }
                }
                // Inside a fork branch, update branch end_next_slot
                if let Some(fork_frame) = fork_stack.last_mut() {
                    let bi = fork_frame.current_branch;
                    fork_frame.branches[bi].end_next_slot = next_slot_y;
                }
                current_slot_y = next_slot_y;
            }
        }
    }

    // Total height needed
    let height = node_layouts
        .iter()
        .map(|l| l.next_slot_y)
        .max()
        .unwrap_or(header_h + step_h)
        + 60;

    // ---------------------------------------------------------------------------
    // Emit SVG
    // ---------------------------------------------------------------------------
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&act_style.background_color)
    ));

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                y_cursor,
                escape_text(&act_style.font_color),
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">activity diagram</text>",
        y_cursor + 2,
        escape_text(&act_style.font_color)
    ));

    let lane_left = |idx: i32| -> i32 { lane_area_x + idx * lane_w };

    for (idx, lane) in lanes.iter().enumerate() {
        let lx = lane_left(idx as i32);
        let bg = if idx % 2 == 0 {
            act_style.background_color.as_str()
        } else {
            "#f1f5f9"
        };
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
            lx,
            header_h - 8,
            lane_w,
            height - header_h - 20,
            bg
        ));
        if lane != "default" {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                lx + lane_w / 2,
                header_h + 10,
                escape_text(&act_style.font_color),
                escape_text(lane)
            ));
        }
    }

    let box_w = (lane_w - 24).clamp(120, 220);

    // ---------------------------------------------------------------------------
    // Pass 2: render nodes and arrows using pre-computed positions
    // ---------------------------------------------------------------------------
    for (i, (node, meta)) in doc.nodes.iter().zip(metas.iter()).enumerate() {
        let layout = &node_layouts[i];
        let cx = layout.cx;
        let y = layout.slot_y;
        let label = node.label.clone().unwrap_or_default();
        let step_kind = &meta.step_kind;
        let fork_branch = meta.fork_branch;

        out.push_str(&format!(
            "<metadata data-activity-kind=\"{}\" data-activity-lane=\"{}\" data-activity-branch=\"{}\"/>",
            escape_text(step_kind),
            escape_text(&meta.lane_name),
            fork_branch
        ));
        match node.kind {
            FamilyNodeKind::ActivityStart => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\"/>",
                    cx,
                    y + 20,
                    act_style.fork_color
                ));
            }
            FamilyNodeKind::ActivityStop => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 20,
                    act_style.fork_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"{}\"/>",
                    cx,
                    y + 20,
                    act_style.fork_color
                ));
                if !label.is_empty() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                        cx,
                        y + 44,
                        escape_text(&act_style.font_color),
                        escape_text(&label)
                    ));
                }
            }
            FamilyNodeKind::ActivityAction => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx - box_w / 2,
                    y + 4,
                    box_w,
                    act_style.background_color,
                    act_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            }
            FamilyNodeKind::Note => {
                render_note_card(&mut out, cx - box_w / 2, y + 2, box_w, 44, &label);
            }
            FamilyNodeKind::ActivityDecision => {
                // diamond
                // Split "condition / guard" — condition inside diamond, guard floats
                // on the outgoing "then" arrow (right side of diamond).
                let (condition_text, then_guard) = if let Some(idx) = label.find(" / ") {
                    (&label[..idx], Some(&label[idx + 3..]))
                } else {
                    (label.as_str(), None)
                };
                let dx = 100;
                let dy = 22;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 2,
                    cx + dx,
                    y + 2 + dy,
                    cx,
                    y + 2 + (dy * 2),
                    cx - dx,
                    y + 2 + dy,
                    act_style.diamond_color,
                    act_style.border_color
                ));
                // Condition text inside the diamond
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    cx,
                    y + 2 + dy + 4,
                    escape_text(&act_style.font_color),
                    escape_text(condition_text)
                ));
                // Guard label floats at the right tip of the diamond (then-branch side)
                if let Some(guard) = then_guard {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
                        cx + dx + 4,
                        y + 2 + dy + 4,
                        escape_text(&act_style.font_color),
                        escape_text(guard)
                    ));
                }
                if step_kind.contains("WhileStart") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">while</text>",
                        cx,
                        y + 54,
                        escape_text(&act_style.font_color)
                    ));
                }
                if step_kind.contains("RepeatWhile") {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">repeat while</text>",
                        cx,
                        y + 54,
                        escape_text(&act_style.font_color)
                    ));
                }
            }
            FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                if step_kind.contains("ForkAgain") {
                    // ForkAgain is a separator — draw a small tick at branch column position.
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"3 2\"/>",
                        cx - 16, y + 28, cx + 16, y + 28,
                        escape_text(&act_style.fork_color)
                    ));
                } else {
                    // Fork or EndFork bar: use precomputed span
                    let bar_half = fork_bar_half_widths.get(&i).copied().unwrap_or(box_w / 2);
                    let bar_w = (bar_half * 2).max(box_w);
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"{}\"/>",
                        cx - bar_w / 2,
                        y + 24,
                        bar_w,
                        act_style.fork_color
                    ));
                }
            }
            FamilyNodeKind::ActivityMerge => {
                // Else and EndIf are invisible control-flow markers — they
                // drive arrow routing but must never appear as visible text.
                if step_kind.contains("Else") || step_kind.contains("EndIf") {
                    // no visual output — layout only
                } else {
                    let merge_label = if step_kind.contains("EndWhile") {
                        if label.is_empty() {
                            "(endwhile)".to_string()
                        } else {
                            format!("({label})")
                        }
                    } else if step_kind.contains("RepeatStart") {
                        "(repeat)".to_string()
                    } else {
                        format!("(merge) {}", label)
                    };
                    if !merge_label.is_empty() {
                        out.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                            cx,
                            y + 28,
                            escape_text(&act_style.font_color),
                            escape_text(&merge_label)
                        ));
                    }
                }
            }
            FamilyNodeKind::ActivityPartition => {
                out.push_str(&format!(
                    "<rect x=\"24\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                    y + 4,
                    width - 48,
                    escape_text(&act_style.background_color),
                    escape_text(&act_style.border_color)
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"{}\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&act_style.font_color),
                    escape_text(&format!("partition: {}", label))
                ));
            }
            _ => {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&act_style.font_color),
                    escape_text(&label)
                ));
            }
        }

        // Arrow from previous node (suppressed for branch-control nodes)
        if i > 0 && !suppress_prev_arrow.contains(&i) {
            let prev = &node_layouts[i - 1];
            let (px, py) = (prev.cx, prev.arrow_out_y);
            emit_activity_arrow(&mut out, px, py, cx, y, &act_style.arrow_color);
        }

        // Extra arrows for if-branching and fork connections
        for (x1, y1, x2, y2) in extra_arrows.iter().filter(|a| a.2 == cx && a.3 == y) {
            emit_activity_arrow(&mut out, *x1, *y1, *x2, *y2, &act_style.arrow_color);
        }
    }

    out.push_str("</svg>");
    out
}

/// Compute the center X for a fork branch column.
///
/// Branches are laid out symmetrically around `fork_cx`.
/// With N branches and column width `col_w`:
///   total span = (N-1) * col_w
///   leftmost branch center = fork_cx - (N-1)*col_w/2
///   branch k center = leftmost + k * col_w
fn fork_branch_cx(fork_cx: i32, branch_idx: usize, n_branches: usize, col_w: i32) -> i32 {
    if n_branches <= 1 {
        return fork_cx;
    }
    let total_span = (n_branches as i32 - 1) * col_w;
    let leftmost = fork_cx - total_span / 2;
    leftmost + branch_idx as i32 * col_w
}

/// Emit a straight arrow from (x1,y1) to (x2,y2) with an arrowhead at (x2,y2).
fn emit_activity_arrow(out: &mut String, x1: i32, y1: i32, x2: i32, y2: i32, color: &str) {
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x1, y1, x2, y2, color
    ));
    // Arrowhead: small triangle pointing in the direction of travel
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = ((dx * dx + dy * dy) as f64).sqrt().max(1.0);
    let ux = dx as f64 / len;
    let uy = dy as f64 / len;
    // Perpendicular
    let px = -uy;
    let py = ux;
    let tip_x = x2 as f64;
    let tip_y = y2 as f64;
    let base_x = tip_x - ux * 8.0;
    let base_y = tip_y - uy * 8.0;
    let l_x = (base_x + px * 4.0).round() as i32;
    let l_y = (base_y + py * 4.0).round() as i32;
    let r_x = (base_x - px * 4.0).round() as i32;
    let r_y = (base_y - py * 4.0).round() as i32;
    out.push_str(&format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
        x2, y2, l_x, l_y, r_x, r_y, color
    ));
}

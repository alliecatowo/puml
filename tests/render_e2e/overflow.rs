use super::support::*;
use puml::scene::LayoutOptions;
use puml::{layout, render};
use std::collections::HashSet;

#[test]
fn overflow_scene_text_anchors_stay_within_note_and_group_bounds() {
    let src = fixture("overflow/overflow_notes_refs_groups.puml");
    let ast = puml::parse(&src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());

    for note in &scene.notes {
        for (idx, _) in note.text.lines().enumerate() {
            let text_y = note.y + 20 + (idx as i32 * 16);
            assert!(
                text_y > note.y && text_y <= note.y + note.height,
                "note text baseline should stay within note rect bounds"
            );
        }
    }

    for group in &scene.groups {
        if let Some(label) = &group.label {
            let header_y = group.y + 16;
            assert!(
                header_y > group.y && header_y <= group.y + group.height,
                "group header baseline should stay within group rect bounds"
            );
            if group.kind.eq_ignore_ascii_case("ref") {
                for (idx, _) in label.lines().skip(1).enumerate() {
                    let text_y = group.y + 32 + (idx as i32 * 16);
                    assert!(
                        text_y > group.y && text_y <= group.y + group.height,
                        "ref body baseline should stay within ref rect bounds"
                    );
                }
            }
        }
    }
}

#[test]
fn overflow_svg_text_positions_stay_within_associated_rects() {
    let src = fixture("overflow/overflow_notes_refs_groups.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let note_rects = rects
        .iter()
        .filter(|r| r.fill == "#fff8c4")
        .collect::<Vec<_>>();
    let group_rects = rects
        .iter()
        .filter(|r| r.fill == "#eef6ff" || r.fill == "#fafafa")
        .collect::<Vec<_>>();
    assert!(!note_rects.is_empty(), "expected at least one note rect");
    assert!(
        !group_rects.is_empty(),
        "expected at least one group/ref rect"
    );

    let tracked = [
        "note_line_one_for_bounds_guardrail",
        "note_line_two_for_bounds_guardrail",
        "note_line_three_for_bounds_guardrail",
        "alt branch_label_for_bounds_guardrail",
        "ref_line_one_for_bounds_guardrail",
        "ref_line_two_for_bounds_guardrail",
        "ref_line_three_for_bounds_guardrail",
        "ref_line_four_for_bounds_guardrail",
    ];

    let mut seen = HashSet::new();
    for text in texts {
        if !tracked.contains(&text.text.as_str()) {
            continue;
        }
        seen.insert(text.text.clone());
        let owner = note_rects
            .iter()
            .copied()
            .chain(group_rects.iter().copied())
            .find(|r| {
                text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
            });
        assert!(
            owner.is_some(),
            "tracked text should stay inside associated note/ref/group rect bounds: {}",
            text.text
        );
    }

    for expected in tracked {
        assert!(
            seen.contains(expected),
            "expected tracked overflow guardrail text in svg: {expected}"
        );
    }
}

#[test]
fn class_stereotype_edge_labels_clear_adjacent_class_boxes() {
    let src = r#"@startuml
class UserController <<controller>>
class UserService <<service>>
class UserRepository <<repository>>
class User <<entity>>
UserController --> UserService : delegates
UserService --> UserRepository : persists
UserRepository --> User : maps
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("render should succeed");
    let mut outer_rects = parse_svg_rects(&svg)
        .into_iter()
        .filter(|rect| rect.fill == "#ffffff" && rect.width == 160 && rect.height == 52)
        .collect::<Vec<_>>();
    outer_rects.sort_by_key(|rect| rect.y);
    assert!(
        outer_rects.len() >= 3,
        "expected at least three class outer rects, got {}",
        outer_rects.len()
    );

    let texts = parse_svg_texts(&svg);
    for (label, upper_idx, lower_idx) in [("delegates", 0usize, 1usize), ("persists", 1, 2)] {
        let text = texts
            .iter()
            .find(|text| text.text == label)
            .unwrap_or_else(|| panic!("missing relation label {label}"));
        let clearance_left = text.x - ((label.chars().count() as i32) * 3).max(18);
        let blocked_right_edge = (outer_rects[upper_idx].x + outer_rects[upper_idx].width)
            .max(outer_rects[lower_idx].x + outer_rects[lower_idx].width);
        assert!(
            clearance_left >= blocked_right_edge + 8,
            "label {label} should clear adjacent class boxes by 8px: left edge {clearance_left}, blocked edge {}",
            blocked_right_edge
        );
    }
}

#[test]
fn render_svg_wraps_long_message_labels_without_viewbox_clipping() {
    let src = fixture("overflow/overflow_message_labels.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.contains("LEFTE"));
    assert!(svg.contains("CENTEROVERFLOWTOKEN"));
    assert!(svg.contains("RIGHT"));
    assert_snapshot!(
        "render_svg_wraps_long_message_labels_without_viewbox_clipping",
        svg
    );
}

#[test]
fn overflow_message_label_positions_stay_within_scene_viewbox() {
    let src = fixture("overflow/overflow_message_labels.puml");
    let ast = puml::parse(&src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());

    for message in &scene.messages {
        if message.label_lines.is_empty() {
            continue;
        }
        let tx = ((message.x1 + message.x2) / 2) + 2;
        let start_y =
            message.y - 8 - (((message.label_lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP);
        for (idx, line) in message.label_lines.iter().enumerate() {
            let width = (line.chars().count() as i32) * 7;
            let left = tx - (width / 2);
            let right = tx + (width / 2);
            let y = start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP);

            assert!(left >= 0, "message label left edge should be in viewBox");
            assert!(
                right <= scene.width,
                "message label right edge should be in viewBox"
            );
            assert!(y >= 0, "message label baseline should be in viewBox");
            assert!(
                y <= scene.height,
                "message label baseline should be in viewBox"
            );
        }
    }
}

#[test]
fn overflow_unbroken_tokens_stay_within_note_and_ref_rects() {
    let src = fixture("overflow/overflow_unbroken_tokens.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let note_rects = rects
        .iter()
        .filter(|r| r.fill == "#fff8c4")
        .collect::<Vec<_>>();
    let ref_rects = rects
        .iter()
        .filter(|r| r.fill == "#eef6ff")
        .collect::<Vec<_>>();

    assert!(!note_rects.is_empty(), "expected note rects");
    assert!(!ref_rects.is_empty(), "expected ref rects");

    let tracked = [
        "note_unbroken_token_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "ref_unbroken_token_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    ];

    for token in tracked {
        let text = texts
            .iter()
            .find(|t| t.text == token)
            .unwrap_or_else(|| panic!("expected token in svg: {token}"));
        let owner = note_rects
            .iter()
            .copied()
            .chain(ref_rects.iter().copied())
            .find(|r| {
                text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
            });
        assert!(
            owner.is_some(),
            "unbroken overflow token should stay inside note/ref bounds: {token}"
        );
    }

    assert_snapshot!(
        "overflow_unbroken_tokens_stay_within_note_and_ref_rects",
        svg
    );
}

#[test]
fn overflow_advanced_note_ref_forms_do_not_overlap_and_render_deterministically() {
    let src = fixture("overflow/overflow_note_ref_advanced_forms_nonoverlap.puml");
    let ast = puml::parse(&src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let mut blocks = Vec::new();
    for note in &scene.notes {
        blocks.push(("note", note.y, note.y + note.height));
    }
    for group in &scene.groups {
        if group.kind.eq_ignore_ascii_case("ref") {
            blocks.push(("ref", group.y, group.y + group.height));
        }
    }

    blocks.sort_by_key(|(_, y, _)| *y);
    for window in blocks.windows(2) {
        let (first_kind, _first_y, first_bottom) = window[0];
        let (second_kind, second_y, _second_bottom) = window[1];
        assert!(
            second_y >= first_bottom,
            "advanced annotation boxes should not overlap: {first_kind} bottom {} > {second_kind} top {}",
            first_bottom,
            second_y
        );
    }

    let svg = render::render_svg(&scene);
    let rerendered = puml::render_source_to_svg(&src).expect("render should succeed");
    assert_eq!(svg, rerendered, "render output should be deterministic");
    assert_snapshot!(
        "overflow_advanced_note_ref_forms_do_not_overlap_and_render_deterministically",
        svg
    );
}

#[test]
fn overflow_multiline_group_ref_note_combo_stays_within_rects() {
    let src = fixture("overflow/overflow_multiline_group_ref_note_combo.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let note_rects = rects
        .iter()
        .filter(|r| r.fill == "#fff8c4")
        .collect::<Vec<_>>();
    let group_rects = rects
        .iter()
        .filter(|r| r.fill == "#eef6ff" || r.fill == "#fafafa")
        .collect::<Vec<_>>();
    let viewbox_width = parse_svg_viewbox_width(&svg).expect("svg should include viewBox width");

    assert!(!note_rects.is_empty(), "expected note rects");
    assert!(!group_rects.is_empty(), "expected group/ref rects");

    let tracked = [
        "combo_note_line_1_with_a_very_long_unbroken_token_CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        "combo_ref_line_1_with_a_very_long_unbroken_token_DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
        "fallback_note_line_1_with_long_unbroken_token_EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE",
    ];

    for line in tracked {
        let text = texts
            .iter()
            .find(|t| t.text == line)
            .unwrap_or_else(|| panic!("expected combo overflow text in svg: {line}"));
        let owner = note_rects
            .iter()
            .copied()
            .chain(group_rects.iter().copied())
            .find(|r| {
                text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
            });
        assert!(
            owner.is_some(),
            "combo overflow text should stay within associated rects: {line}"
        );
        if let Some(note_rect) = note_rects
            .iter()
            .copied()
            .find(|r| text.x >= r.x && text.y > r.y && text.y <= r.y + r.height)
        {
            let conservative_right = text.x + ((text.text.chars().count() as i32) * 7);
            assert!(
                conservative_right <= note_rect.x + note_rect.width,
                "long note text should fit note rect width without right-edge clipping: {line}"
            );
            assert!(
                conservative_right <= viewbox_width,
                "long note text should fit scene viewBox width without right-edge clipping: {line}"
            );
        }
    }

    assert_snapshot!(
        "overflow_multiline_group_ref_note_combo_stays_within_rects",
        svg
    );
}

#[test]
fn overflow_dense_participant_headers_keep_text_inside_header_boxes() {
    let src = fixture("overflow/overflow_dense_participant_headers.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let participant_rects = rects
        .iter()
        .filter(|r| r.fill == "#f6f6f6")
        .collect::<Vec<_>>();
    assert!(
        participant_rects.len() >= 6,
        "expected participant header and footbox rects"
    );

    let tracked_prefixes = [
        "ParticipantHeaderAlpha",
        "ParticipantHeaderBeta",
        "ParticipantHeaderGamma",
        "ParticipantHeaderDelta",
        "ParticipantHeaderEpsilon",
        "ParticipantHeaderZeta",
    ];

    for text in texts {
        if !tracked_prefixes.iter().any(|p| text.text.starts_with(p)) {
            continue;
        }
        let owner = participant_rects.iter().copied().find(|r| {
            text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
        });
        assert!(
            owner.is_some(),
            "dense participant header text should stay inside participant box: {}",
            text.text
        );
    }

    assert_snapshot!(
        "overflow_dense_participant_headers_keep_text_inside_header_boxes",
        svg
    );
}

#[test]
fn lifelines_start_below_wrapped_participant_headers() {
    let src = "@startuml\nparticipant \"Participant Header With Many Wrapped Words For Height Growth\" as P\nP -> P: ping\n@enduml\n";
    let doc = puml::parse(src).expect("parse");
    let model = puml::normalize(doc).expect("normalize");
    let scene = layout::layout(&model, LayoutOptions::default());

    let participant = scene
        .participants
        .iter()
        .find(|p| p.id == "P")
        .expect("participant");
    let lifeline = scene
        .lifelines
        .iter()
        .find(|l| l.participant_id == "P")
        .expect("lifeline");

    assert_eq!(
        lifeline.y1,
        participant.y + participant.height,
        "lifeline should start at participant box bottom"
    );
}

/// Regression test for #731: a self-loop inside a break/group block must not
/// overlap the label of the immediately following message.  The self-loop
/// U-shape extends SELF_LOOP_DROP (32 px) below its `y`; the next message's
/// label is placed 8 px above its own `y`.  Adequate row allocation for the
/// self-loop must ensure the following message label is strictly below the
/// loop bottom.
#[test]
fn regression_731_self_loop_in_break_block_clears_following_message_label() {
    let src = "@startuml\nAlice -> Bob: request\nbreak on error\n  Alice -> Alice: abort\n  Alice -> Bob: cleanup\nend\nBob -> Alice: response\n@enduml\n";
    let doc = puml::parse(src).expect("parse");
    let model = puml::normalize(doc).expect("normalize");
    let scene = layout::layout(&model, LayoutOptions::default());

    let self_loop = scene
        .messages
        .iter()
        .find(|m| m.from_id == m.to_id && m.label.as_deref() == Some("abort"))
        .expect("abort self-loop must be in the scene");

    let cleanup = scene
        .messages
        .iter()
        .find(|m| m.label.as_deref() == Some("cleanup"))
        .expect("cleanup message must follow the self-loop");

    // SELF_LOOP_DROP = 32 px (must match constant in layout.rs / render/sequence.rs)
    let self_loop_bottom = self_loop.route_y + 32;
    // label of next message sits 8 px above its line
    let cleanup_label_top = cleanup.route_y - 8;

    assert!(
        cleanup_label_top > self_loop_bottom,
        "cleanup label top (y={cleanup_label_top}) must be strictly below self-loop bottom \
        (y={self_loop_bottom}); self-loop overlapping following message label (issue #731)"
    );
}

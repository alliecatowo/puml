//! Integration tests for the Phase A `<style>` block parser (issue #1413).
//!
//! Each test exercises a realistic PlantUML `<style>` fragment drawn from
//! upstream themes (cyborg, hacker, reddress-*) or from the PlantUML LRG.
//! The tests assert:
//! 1. The typed `StyleBlock` AST captures selector chains and properties.
//! 2. The compat shim still emits legacy `StatementKind::StyleParam` triples.
//! 3. Round-trip: parse → re-parse equals the original rule count.

use puml::ast::style::{PName, SName, SelectorSegment, StyleScheme};
use puml::parser::style_block::parse_style_block_body;

// ---------------------------------------------------------------------------
// Helper: parse body text and return the StyleBlock.
// ---------------------------------------------------------------------------
fn parse(src: &str) -> puml::ast::style::StyleBlock {
    let (block, _) = parse_style_block_body(src);
    block
}

fn parse_full(
    src: &str,
) -> (
    puml::ast::style::StyleBlock,
    Vec<puml::parser::style_block::CompatTriple>,
) {
    parse_style_block_body(src)
}

// ---------------------------------------------------------------------------
// 1. Cyborg root-block flat properties
// ---------------------------------------------------------------------------
/// Parse the root `{ ... }` section from the cyborg theme.
#[test]
fn cyborg_root_block() {
    let src = r#"
  root {
    BackgroundColor transparent
    FontColor #FFFFFF
    HyperLinkColor #fd7e14
    LineColor #55B2DE
    LineThickness 1
    Margin 10
    Padding 6
    Shadowing 0.0
  }
"#;
    let block = parse(src);
    assert!(
        !block.rules.is_empty(),
        "cyborg root block must produce at least one rule"
    );
    let has_bg = block
        .rules
        .iter()
        .any(|r| r.properties.contains_key(&PName::BackgroundColor));
    assert!(has_bg, "BackgroundColor must be captured");
    let has_shadow = block
        .rules
        .iter()
        .any(|r| r.properties.contains_key(&PName::Shadowing));
    assert!(has_shadow, "Shadowing must be captured");
    let has_link = block
        .rules
        .iter()
        .any(|r| r.properties.contains_key(&PName::HyperLinkColor));
    assert!(has_link, "HyperLinkColor must be captured");
}

// ---------------------------------------------------------------------------
// 2. Cyborg nwdiagDiagram nested block
// ---------------------------------------------------------------------------
/// Two-level nesting: `nwdiagDiagram { group { … } }`.
#[test]
fn cyborg_nwdiag_nested() {
    let src = r#"
  nwdiagDiagram {
    network {
      LineColor #2A9FD6
      LineThickness 1.0
      FontColor #55B2DE
    }
    server {
      BackgroundColor #2A9FD6
    }
    arrow {
      FontColor #55B2DE
      LineColor #55B2DE
    }
    group {
      BackGroundColor #222222
      LineColor #444444
      LineThickness 2.0
      Margin 5
      Padding 5
    }
  }
"#;
    let block = parse(src);
    // All rules must have nwdiagDiagram as their outermost selector.
    for rule in &block.rules {
        let outer = rule
            .selector_path
            .first()
            .expect("rules must have at least one path entry");
        let first_seg = outer
            .segments
            .first()
            .expect("chain must have at least one segment");
        assert_eq!(
            *first_seg,
            SelectorSegment::Tag(SName::NwdiagDiagram),
            "outer selector must be NwdiagDiagram, got {first_seg:?}"
        );
    }
    // group rule must exist with BackgroundColor
    let group_with_bg = block.rules.iter().any(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Tag(SName::Group)))
        }) && r.properties.contains_key(&PName::BackgroundColor)
    });
    assert!(group_with_bg, "group rule must carry BackgroundColor");
}

// ---------------------------------------------------------------------------
// 3. Hacker mindmapDiagram + wbsDiagram comma selectors
// ---------------------------------------------------------------------------
/// `mindmapDiagram, wbsDiagram { element { … } :depth(0) { … } }`
#[test]
fn hacker_mindmap_wbs_comma() {
    let src = r#"
mindmapDiagram, wbsDiagram {
    element {
        BackgroundColor #77B300
        FontColor #FFFFFF
    }
    :depth(0) {
        fontSize 16
        fontStyle bold
    }
    :depth(1) {
        BackgroundColor #333333
    }
}
"#;
    let block = parse(src);
    // At least one depth rule should appear
    let depth0 = block.rules.iter().any(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Depth(0)))
        })
    });
    assert!(depth0, ":depth(0) rule must be present");
    // element rules must appear
    let elem_rule = block.rules.iter().any(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Tag(SName::Element)))
        }) && r.properties.contains_key(&PName::BackgroundColor)
    });
    assert!(elem_rule, "element rule must have BackgroundColor");
}

// ---------------------------------------------------------------------------
// 4. Cyborg ganttDiagram multi-level nested
// ---------------------------------------------------------------------------
/// Three sub-selectors under `ganttDiagram`.
#[test]
fn cyborg_gantt_nested() {
    let src = r#"
  ganttDiagram {
    task {
      BackGroundColor #2A9FD6
      LineColor #2A9FD6
      Margin 10
      Padding 6
    }
    note {
      FontColor #FFFFFF
      LineColor #AD5CD6
      BackGroundColor #9933CC
    }
    separator {
      LineColor #555555
      BackGroundColor #555555-#777777
      FontColor #FFFFFF
    }
    milestone {
      FontColor #9933CC
      FontSize 16
      FontStyle italic
      BackGroundColor #555555
      LineColor #777777
    }
    timeline {
      BackgroundColor #555555
      FontColor #FFFFFF
    }
    closed {
      BackgroundColor #FFA033
      FontColor #FFFFFF
    }
  }
"#;
    let block = parse(src);
    let task_rule = block.rules.iter().find(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Tag(SName::Task)))
        })
    });
    assert!(task_rule.is_some(), "task rule must be present");
    let milestone_rule = block.rules.iter().find(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Tag(SName::Milestone)))
        })
    });
    assert!(milestone_rule.is_some(), "milestone rule must be present");
    let timeline_rule = block.rules.iter().find(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Tag(SName::Timeline)))
        })
    });
    assert!(timeline_rule.is_some(), "timeline rule must be present");
}

// ---------------------------------------------------------------------------
// 5. Stereotype selector (.Apache)
// ---------------------------------------------------------------------------
#[test]
fn stereotype_selector_parses() {
    let src = r#"
.Apache {
  BackgroundColor #FF6600
  FontColor #FFFFFF
  LineColor #CC4400
}
"#;
    let block = parse(src);
    let stereo_rule = block.rules.iter().find(|r| {
        r.selector_path.iter().any(|c| {
            c.segments
                .iter()
                .any(|s| matches!(s, SelectorSegment::Stereotype(n) if n == "Apache"))
        })
    });
    assert!(stereo_rule.is_some(), ".Apache stereotype rule must parse");
    assert!(
        stereo_rule
            .unwrap()
            .properties
            .contains_key(&PName::BackgroundColor),
        "stereotype rule must capture BackgroundColor"
    );
}

// ---------------------------------------------------------------------------
// 6. @media dark block — rules tagged Dark scheme
// ---------------------------------------------------------------------------
#[test]
fn at_media_dark_scheme_tag() {
    let src = r#"
root {
  BackgroundColor white
}
@media dark {
  root {
    BackgroundColor #1a1a1a
    FontColor #CCCCCC
  }
}
"#;
    let block = parse(src);
    let dark_rule = block.rules.iter().find(|r| {
        r.scheme == StyleScheme::Dark && r.properties.contains_key(&PName::BackgroundColor)
    });
    assert!(
        dark_rule.is_some(),
        "@media dark rule must be tagged Dark scheme"
    );
    let light_rule = block.rules.iter().find(|r| {
        r.scheme == StyleScheme::Regular && r.properties.contains_key(&PName::BackgroundColor)
    });
    assert!(
        light_rule.is_some(),
        "regular rule must remain Regular scheme"
    );
}

// ---------------------------------------------------------------------------
// 7. CSS variable declarations stored in `variables` map
// ---------------------------------------------------------------------------
#[test]
fn css_variables_stored() {
    let src = r#"
--primary: #2A9FD6
--secondary: #555555
root {
  BackgroundColor transparent
}
"#;
    let block = parse(src);
    assert_eq!(
        block.variables.get("--primary").map(String::as_str),
        Some("#2A9FD6"),
        "--primary CSS var must be stored"
    );
    assert!(
        block.variables.contains_key("--secondary"),
        "--secondary CSS var must be stored"
    );
}

// ---------------------------------------------------------------------------
// 8. Compat shim: legacy StyleParam triples still emitted
// ---------------------------------------------------------------------------
/// Any `<style>` block processed by the new parser MUST still produce legacy
/// flat triples so existing per-family resolvers continue to work unchanged.
#[test]
fn compat_shim_emits_legacy_triples() {
    let src = r#"
participant {
  BackgroundColor #AABBCC
  BorderColor #001122
  FontColor #FFFFFF
}
note {
  BackgroundColor #FFEEAA
}
"#;
    let (_block, compat) = parse_full(src);
    assert!(
        !compat.is_empty(),
        "compat shim must emit at least one triple"
    );
    let bg_triple = compat
        .iter()
        .find(|t| t.property.eq_ignore_ascii_case("backgroundcolor"));
    assert!(
        bg_triple.is_some(),
        "BackgroundColor must appear as a compat triple"
    );
}

// ---------------------------------------------------------------------------
// 9. Round-trip: parse → capture rule count → re-parse equals original
// ---------------------------------------------------------------------------
#[test]
fn round_trip_rule_count_stable() {
    let src = r#"
sequenceDiagram {
  participant {
    BackgroundColor #AABBCC
    FontColor #FFFFFF
  }
  note {
    BackgroundColor #FFEEAA
    BorderColor #CCCC00
  }
  arrow {
    LineColor #666666
  }
}
"#;
    let block1 = parse(src);
    let count1 = block1.rules.len();
    // Re-parse must produce the same rule count
    let block2 = parse(src);
    let count2 = block2.rules.len();
    assert_eq!(count1, count2, "rule count must be stable across re-parse");
    assert!(
        count1 >= 3,
        "should have at least 3 rules (participant, note, arrow)"
    );
}

// ---------------------------------------------------------------------------
// 10. Unknown properties are stored, not silently dropped
// ---------------------------------------------------------------------------
#[test]
fn unknown_properties_preserved() {
    let src = r#"
title {
  BorderRoundCorner 8
  BorderThickness 1
  SomeFutureProperty foobar
}
"#;
    let block = parse(src);
    // At least one rule for title
    let title_rule = block.rules.iter().find(|r| {
        r.properties.contains_key(&PName::RoundCorner)
            || r.properties.contains_key(&PName::LineThickness)
            || r.unknown_properties.contains_key("SomeFutureProperty")
    });
    assert!(
        title_rule.is_some(),
        "title rule should parse including unknown properties"
    );
    let has_unknown = block
        .rules
        .iter()
        .any(|r| r.unknown_properties.contains_key("SomeFutureProperty"));
    assert!(
        has_unknown,
        "SomeFutureProperty should be preserved in unknown_properties"
    );
}

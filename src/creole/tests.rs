use super::*;

fn single_line(text: &str) -> CreoleLine {
    let lines = tokenize_creole(text);
    assert_eq!(lines.len(), 1);
    lines.into_iter().next().unwrap()
}

#[test]
fn plain_text_is_single_span() {
    let line = single_line("hello world");
    assert_eq!(line.len(), 1);
    assert_eq!(line[0].text, "hello world");
    assert!(!line[0].bold);
}

#[test]
fn bold_toggles_state() {
    let line = single_line("**bold** plain");
    assert_eq!(line[0].text, "bold");
    assert!(line[0].bold);
    assert_eq!(line[1].text, " plain");
    assert!(!line[1].bold);
}

#[test]
fn italic_toggles_state() {
    let line = single_line("//italic// text");
    assert!(line[0].italic);
    assert!(!line[1].italic);
}

#[test]
fn mono_toggles_state() {
    let line = single_line("\"\"code\"\" text");
    assert!(line[0].mono);
    assert!(!line[1].mono);
}

#[test]
fn underline_toggles_state() {
    let line = single_line("__ul__ text");
    assert!(line[0].underline);
    assert!(!line[1].underline);
}

#[test]
fn strike_toggles_state() {
    let line = single_line("--strike-- text");
    assert!(line[0].strike);
    assert!(!line[1].strike);
}

#[test]
fn link_with_label() {
    let line = single_line("[[https://example.com click me]]");
    assert_eq!(line[0].link.as_deref(), Some("https://example.com"));
    assert_eq!(line[0].text, "click me");
    assert!(line[0].underline);
}

#[test]
fn link_without_label_uses_url() {
    let line = single_line("[[https://example.com]]");
    assert_eq!(line[0].link.as_deref(), Some("https://example.com"));
    assert_eq!(line[0].text, "https://example.com");
}

#[test]
fn color_tag() {
    let line = single_line("<color:red>text</color>");
    assert_eq!(line[0].color.as_deref(), Some("red"));
    assert_eq!(line[0].text, "text");
}

#[test]
fn hex_color_tag() {
    let line = single_line("<color:#FF0000>red</color>");
    assert_eq!(line[0].color.as_deref(), Some("#FF0000"));
}

#[test]
fn size_tag() {
    let line = single_line("<size:18>big</size>");
    assert_eq!(line[0].size, Some(18));
    assert_eq!(line[0].text, "big");
}

#[test]
fn html_bold_tag() {
    let line = single_line("<b>bold</b> plain");
    assert!(line[0].bold);
    assert!(!line[1].bold);
}

#[test]
fn html_italic_tag() {
    let line = single_line("<i>italic</i> plain");
    assert!(line[0].italic);
    assert!(!line[1].italic);
}

#[test]
fn html_underline_tag() {
    let line = single_line("<u>ul</u> plain");
    assert!(line[0].underline);
    assert!(!line[1].underline);
}

#[test]
fn newline_splits_into_multiple_lines() {
    let lines = tokenize_creole("line1\nline2");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0][0].text, "line1");
    assert_eq!(lines[1][0].text, "line2");
}

#[test]
fn backslash_n_splits_into_multiple_lines() {
    // \n in the source string (the two characters \ and n)
    let lines = tokenize_creole(r"line1\nline2");
    assert_eq!(lines.len(), 2);
}

#[test]
fn icon_placeholder() {
    let line = single_line("<&home>");
    assert_eq!(line[0].text, "[home]");
}

#[test]
fn mixed_bold_italic_nesting() {
    let line = single_line("**bold //bi//** only bold");
    // "bold //bi//" is bold; "bi" is bold+italic; " only bold" is plain
    // But since we parse sequentially, order matters.
    let bold_span = line.iter().find(|s| s.text == "bold ");
    assert!(bold_span.is_some_and(|s| s.bold));
}

#[test]
fn render_bold_span() {
    let lines = tokenize_creole("**hi**");
    let out = render_creole_line_to_tspans(&lines[0], 0, "black");
    assert!(out.contains("font-weight=\"bold\""));
    assert!(out.contains(">hi<"));
}

#[test]
fn render_link_span() {
    let lines = tokenize_creole("[[https://x.com go]]");
    let out = render_creole_line_to_tspans(&lines[0], 0, "black");
    assert!(out.contains("xlink:href=\"https://x.com\""));
    assert!(out.contains("fill=\"blue\""));
    assert!(out.contains(">go<"));
}

#[test]
fn render_multi_line_tspans() {
    let lines = tokenize_creole("line1\nline2");
    let out = render_creole_to_svg_tspans(&lines, 10, "black");
    assert!(out.contains("x=\"10\""));
    assert!(out.contains("dy=\"1.2em\""));
}

#[test]
fn decodes_decimal_and_hex_numeric_character_references() {
    assert_eq!(
        decode_unicode_escapes("decimal &#8734; hex &#x221E; upper &#X1F600;"),
        "decimal ∞ hex ∞ upper 😀"
    );
}

#[test]
fn decodes_u_plus_codepoint_tags() {
    assert_eq!(
        decode_unicode_escapes("This is <U+221E> and <u+1F527>"),
        "This is ∞ and 🔧"
    );
}

#[test]
fn decodes_small_emoji_map_and_deterministic_fallback() {
    assert_eq!(
        decode_unicode_escapes("<:calendar:> <:1f600:> <:not_in_small_map:> <#green:sunny:>"),
        "📅 😀 :not_in_small_map: ☀"
    );
}

#[test]
fn leaves_invalid_unicode_escapes_literal() {
    let text = "bad &#xZZ; missing &#9731 no-code <U+110000> no-end <U+221E emoji <::>";
    assert_eq!(decode_unicode_escapes(text), text);
}

#[test]
fn rendered_creole_decodes_escapes_and_removes_escape_text() {
    let lines = tokenize_creole("snow &#9731; infinity <U+221E> <:calendar:>");
    let out = render_creole_line_to_tspans(&lines[0], 0, "black");

    assert!(out.contains("snow ☃ infinity ∞ 📅"));
    assert!(!out.contains("&#9731;"));
    assert!(!out.contains("&lt;U+221E&gt;"));
    assert!(!out.contains("&lt;:calendar:&gt;"));
}

#[test]
fn tilde_escapes_creole_markers() {
    let line = single_line("~**literal** and ~[[x]]");
    assert_eq!(line.len(), 1);
    assert_eq!(line[0].text, "**literal** and [[x]]");
    assert!(!line[0].bold);
    assert!(line[0].link.is_none());
}

#[test]
fn wave_underline_creole_and_html_tag_render() {
    let creole = single_line("~~wave~~");
    assert!(creole[0].wave);

    let html = single_line("<w:red>wave</w>");
    assert!(html[0].wave);
    assert_eq!(html[0].decoration_color.as_deref(), Some("red"));

    let out = render_creole_line_to_tspans(&html, 0, "black");
    assert!(out.contains("text-decoration-style=\"wavy\""));
    assert!(out.contains("text-decoration-color=\"red\""));
}

#[test]
fn link_tooltip_renders_svg_title() {
    let line = single_line("[[https://example.com{Open docs} docs]]");
    assert_eq!(line[0].link.as_deref(), Some("https://example.com"));
    assert_eq!(line[0].link_tooltip.as_deref(), Some("Open docs"));
    assert_eq!(line[0].text, "docs");

    let out = render_creole_line_to_tspans(&line, 0, "black");
    assert!(out.contains("<title>Open docs</title>"));
}

#[test]
fn headings_become_bold_sized_lines() {
    let lines = tokenize_creole("= Main title =\n=== Minor");
    assert_eq!(lines[0][0].text, "Main title");
    assert!(lines[0][0].bold);
    assert_eq!(lines[0][0].size, Some(24));
    assert_eq!(lines[1][0].size, Some(16));
}

#[test]
fn list_lines_add_indented_prefixes_without_triggering_bold() {
    let lines = tokenize_creole("* Bullet\n** Nested\n# Numbered\n## Nested number");
    assert_eq!(lines[0][0].text, "- ");
    assert_eq!(lines[0][1].text, "Bullet");
    assert_eq!(lines[1][0].text, "  - ");
    assert_eq!(lines[2][0].text, "1. ");
    assert_eq!(lines[3][0].text, "  1. ");
    assert!(!lines[1][1].bold);
}

#[test]
fn horizontal_rule_lines_render_as_rule_text() {
    let lines = tokenize_creole("----\n.. Section ..");
    assert_eq!(lines[0][0].text, "------------------------");
    assert!(lines[0][0].mono);
    assert_eq!(lines[1][0].text, "---------- Section ----------");
}

#[test]
fn code_tag_is_verbatim_monospace() {
    let line = single_line("<code>**not bold** & raw</code>");
    assert_eq!(line[0].text, "**not bold** & raw");
    assert!(line[0].mono);
    assert!(!line[0].bold);
}

#[test]
fn table_lines_mark_headers_and_cell_backgrounds() {
    let line = single_line("|= Name |<#FF8080> Value |");
    assert_eq!(line[0].text, "Name");
    assert!(line[0].bold);
    assert!(line[0].mono);
    assert_eq!(line[2].text, "Value");
    assert_eq!(line[2].background.as_deref(), Some("#FF8080"));
}

#[test]
fn row_background_applies_to_table_cells() {
    let line = single_line("<#yellow>| a | b |");
    assert_eq!(line[0].background.as_deref(), Some("yellow"));
    assert_eq!(line[2].background.as_deref(), Some("yellow"));
}

#[test]
fn tree_lines_use_text_tree_prefix() {
    let line = single_line("  |_ child");
    assert_eq!(line[0].text, "  `- ");
    assert!(line[0].mono);
    assert_eq!(line[1].text, "child");
}

#[test]
fn remaining_html_tags_set_span_state() {
    let strike = single_line("<s:green>gone</s>");
    assert!(strike[0].strike);
    assert_eq!(strike[0].decoration_color.as_deref(), Some("green"));

    let plain = single_line("<b><plain>**literal**</plain></b>");
    assert_eq!(plain[0].text, "**literal**");
    assert!(!plain[0].bold);

    let back = single_line("<back:#ffeeaa>highlight</back>");
    assert_eq!(back[0].background.as_deref(), Some("#ffeeaa"));

    let font = single_line("<font:serif>face</font>");
    assert_eq!(font[0].font.as_deref(), Some("serif"));

    let sub = single_line("H<sub>2</sub>O");
    assert_eq!(sub[1].baseline_shift.as_deref(), Some("sub"));

    let sup = single_line("x<sup>2</sup>");
    assert_eq!(sup[1].baseline_shift.as_deref(), Some("super"));
}

#[test]
fn render_remaining_html_tag_attributes() {
    let lines = tokenize_creole("<font:serif><back:yellow><sub>x</sub></back></font> <s>gone</s>");
    let out = render_creole_line_to_tspans(&lines[0], 0, "black");
    assert!(out.contains("font-family=\"serif\""));
    assert!(out.contains("data-creole-back=\"yellow\""));
    assert!(out.contains("baseline-shift=\"sub\""));
    assert!(out.contains("line-through"));
}

use puml::render_source_to_svg;

fn find_text_position(svg: &str, label: &str) -> (i32, i32) {
    let needle = format!(">{label}</text>");
    let end = svg
        .find(&needle)
        .unwrap_or_else(|| panic!("missing text node for label {label}"));
    let start = svg[..end]
        .rfind("<text ")
        .unwrap_or_else(|| panic!("missing <text tag for label {label}"));
    let tag = &svg[start..end];
    let x = parse_attr(tag, "x");
    let y = parse_attr(tag, "y");
    (x, y)
}

fn parse_attr(tag: &str, attr: &str) -> i32 {
    let needle = format!("{attr}=\"");
    let start = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {attr} attribute in {tag}"))
        + needle.len();
    let end = tag[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {attr} attribute in {tag}"))
        + start;
    tag[start..end]
        .parse::<i32>()
        .unwrap_or_else(|_| panic!("non-integer {attr} attribute in {tag}"))
}

#[test]
fn architecture_overview_edge_labels_are_fanned_apart_in_svg_output() {
    let src = include_str!("../docs/diagrams/architecture-overview.puml");
    let svg = render_source_to_svg(src).expect("architecture overview should render");
    let (expanded_source_x, expanded_source_y) = find_text_position(&svg, "expanded source");
    let (parse_with_options_x, parse_with_options_y) =
        find_text_position(&svg, "parse_with_options");
    assert_eq!(expanded_source_y, parse_with_options_y);
    assert!(
        (parse_with_options_x - expanded_source_x).abs() >= 80,
        "parser-lane labels should be clearly fanned apart"
    );

    let (normalized_document_x, normalized_document_y) =
        find_text_position(&svg, "NormalizedDocument");
    let (annotations_x, annotations_y) = find_text_position(&svg, "annotations");
    let (style_tokens_x, style_tokens_y) = find_text_position(&svg, "style tokens");
    assert_eq!(normalized_document_y, annotations_y);
    assert_eq!(annotations_y, style_tokens_y);
    assert!(
        (annotations_x - normalized_document_x).abs() >= 80,
        "renderer inbound labels should be fanned apart on the left"
    );
    assert!(
        (style_tokens_x - annotations_x).abs() >= 80,
        "renderer inbound labels should be fanned apart on the right"
    );
}

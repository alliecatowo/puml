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
fn architecture_overview_preserves_simplified_pipeline_hierarchy() {
    let src = include_str!("../docs/diagrams/architecture-overview.puml");
    let svg = render_source_to_svg(src).expect("architecture overview should render");

    let (cli_x, cli_y) = find_text_position(&svg, "CLI");
    let (lsp_x, lsp_y) = find_text_position(&svg, "LSP");
    let (wasm_x, wasm_y) = find_text_position(&svg, "WASM");
    assert_eq!(cli_y, lsp_y, "transport nodes should share a row");
    assert_eq!(lsp_y, wasm_y, "transport nodes should share a row");
    assert!(
        cli_x < lsp_x && lsp_x < wasm_x,
        "transport nodes should fan left-to-right"
    );

    let (_, adapters_y) = find_text_position(&svg, "Adapters");
    let (_, preprocessor_y) = find_text_position(&svg, "Preprocessor");
    let (_, language_service_y) = find_text_position(&svg, "Language Service");
    assert_eq!(
        adapters_y, preprocessor_y,
        "frontend and preprocess lanes should align"
    );
    assert_eq!(
        preprocessor_y, language_service_y,
        "service entrypoints should align"
    );
    assert!(
        adapters_y > cli_y,
        "frontend/service row should sit below transports"
    );

    let (parser_x, parser_y) = find_text_position(&svg, "Parser");
    let (support_x, support_y) = find_text_position(&svg, "Diagnostics + Theme");
    let (_, ast_y) = find_text_position(&svg, "AST");
    let (_, normalizer_y) = find_text_position(&svg, "Normalizer");
    let (_, renderer_y) = find_text_position(&svg, "Renderer");
    let (_, outputs_y) = find_text_position(&svg, "SVG / PNG / Text");

    assert!(
        parser_y > adapters_y,
        "pipeline core should start below entry rows"
    );
    assert_eq!(
        parser_y, support_y,
        "support component should stay level with the parser lane"
    );
    assert!(
        support_x > parser_x,
        "support component should remain to the right of parser"
    );
    assert!(ast_y > parser_y, "AST should sit below parser");
    assert!(normalizer_y > ast_y, "normalizer should sit below AST");
    assert!(
        renderer_y > normalizer_y,
        "renderer should sit below normalizer"
    );
    assert!(outputs_y > renderer_y, "outputs should sit below renderer");
}

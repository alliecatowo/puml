use super::support::*;
use super::*;

#[test]
fn salt_advanced_widgets_render_tree_menu_tab_scroll_and_table() {
    let src = "@startsalt\n{\n{T\n+ Root\n++ Leaf\n}\n{* File | Edit | View}\n{/ General | Advanced}\n{S vertical 55%}\n| Name | \"Search\" |\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("advanced salt widgets should render");
    assert!(svg.contains("data-salt-widget=\"tree\""));
    assert!(svg.contains("data-salt-widget=\"menu\""));
    assert!(svg.contains("data-salt-widget=\"tab\""));
    assert!(svg.contains("data-salt-widget=\"scrollbar\""));
    assert!(svg.contains("Leaf"));
    assert!(svg.contains("Search"));
}

#[test]
fn salt_style_directives_and_header_cells_affect_widget_svg() {
    let src = "@startsalt\n\
skinparam saltBackgroundColor #f8fafc\n\
skinparam saltPanelColor #ffffff\n\
skinparam saltBorderColor #0f172a\n\
skinparam saltFontColor #111827\n\
skinparam saltHeaderColor #dbeafe\n\
skinparam saltButtonBackgroundColor #bfdbfe\n\
skinparam saltInputBackgroundColor #eff6ff\n\
{\n\
|= Field | = Value |\n\
| Name | \"Ada\" |\n\
| Action | [Save] |\n\
{* File | Edit}\n\
{S horizontal 50%}\n\
}\n\
@endsalt\n";
    let svg = render_source_to_svg(src).expect("styled salt should render");
    assert!(svg.contains("data-salt-style=\"canvas\""));
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#0f172a\""));
    assert!(svg.contains("data-salt-widget=\"header\""));
    assert!(svg.contains("fill=\"#dbeafe\""));
    assert!(svg.contains("fill=\"#bfdbfe\""));
    assert!(svg.contains("fill=\"#eff6ff\""));
    assert!(svg.contains("data-salt-widget=\"menu\""));
    assert!(svg.contains("data-salt-widget=\"scrollbar\""));
    assert!(svg.contains("Field"));
    assert!(svg.contains("Save"));
}

#[test]
fn salt_compact_controls_textarea_advanced_table_and_style_blocks_render() {
    let src = "@startsalt\n\
<style>\n\
saltDiagram {\n\
  BackgroundColor #ecfeff\n\
}\n\
</style>\n\
!option handwritten true\n\
{+\n\
This is a long\n\
text in a textarea\n\
.\n\
\"                         \"\n\
}\n\
{SI\n\
Scrolled notes\n\
}\n\
{#\n\
. | Column 2 | Column 3\n\
Row header 1 | [] unchecked | () radio\n\
Row header 2 | value | *\n\
}\n\
{^ Profile Group}\n\
^Role^ | [Save]\n\
@endsalt\n";
    let svg = render_source_to_svg(src).expect("advanced salt controls should render");
    assert!(svg.contains("data-salt-widget=\"textarea\""));
    assert!(svg.contains("data-salt-scroll-vertical=\"true\""));
    assert!(svg.contains("data-salt-scroll-horizontal=\"false\""));
    assert!(svg.contains("data-salt-widget=\"table-empty\""));
    assert!(svg.contains("data-salt-widget=\"table-span\""));
    assert!(svg.contains("data-salt-widget=\"header\""));
    assert!(svg.contains("data-salt-widget=\"groupbox\""));
    assert!(svg.contains("data-salt-widget=\"scrollbar\""));
    assert!(svg.contains("Comic Sans MS, cursive"));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("unchecked"));
    assert!(svg.contains("radio"));
    assert!(svg.contains("Role"));
}

#[test]
fn salt_creole_icons_sprites_and_scoped_widget_styles_render() {
    let src = "@startsalt\n\
<style>\n\
saltDiagram {\n\
  BackgroundColor #f0fdfa\n\
  FontColor #134e4a\n\
}\n\
button {\n\
  BackgroundColor #fed7aa\n\
  FontColor #7c2d12\n\
}\n\
input {\n\
  BackgroundColor #ecfeff\n\
  FontColor #155e75\n\
}\n\
header {\n\
  BackgroundColor #ccfbf1\n\
  FontColor #115e59\n\
}\n\
menu {\n\
  BackgroundColor #ede9fe\n\
}\n\
tab {\n\
  BackgroundColor #fef3c7\n\
}\n\
scrollbar {\n\
  BackgroundColor #c7d2fe\n\
}\n\
checkbox {\n\
  BackgroundColor #fef9c3\n\
}\n\
radio {\n\
  BackgroundColor #fee2e2\n\
}\n\
</style>\n\
{\n\
|= **Field** | = <color:blue>Value</color> |\n\
| Login<&person> | \"//Ada//\" |\n\
| [] <b>Remember</b> | () <&key> OTP |\n\
| Action | [<b>Save</b> <&account-login>] |\n\
{* File | Edit | Refactor | Open | Close}\n\
{/ <b>General | Advanced}\n\
{SI\n\
<&code> //scroll body//\n\
}\n\
<<folder\n\
.XX.\n\
XXXX\n\
>>\n\
<<folder>> | Done\n\
}\n\
@endsalt\n";
    let svg = render_source_to_svg(src).expect("rich salt style/creole should render");
    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"account-login\""));
    assert!(svg.contains("data-salt-widget=\"sprite\""));
    assert!(svg.contains("data-salt-sprite=\"folder\""));
    assert!(svg.contains("data-salt-widget=\"sprite-ref\""));
    assert!(svg.contains("data-salt-sprite-ref=\"folder\""));
    assert!(svg.contains("fill=\"#fed7aa\""));
    assert!(svg.contains("fill=\"#7c2d12\""));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("fill=\"#155e75\""));
    assert!(svg.contains("fill=\"#ccfbf1\""));
    assert!(svg.contains("fill=\"#115e59\""));
    assert!(svg.contains("fill=\"#ede9fe\""));
    assert!(svg.contains("fill=\"#fef3c7\""));
    assert!(svg.contains("fill=\"#c7d2fe\""));
    assert!(svg.contains("fill=\"#fef9c3\""));
    assert!(svg.contains("fill=\"#fee2e2\""));
    assert!(svg.contains("data-salt-open=\"true\""));
    assert!(svg.contains("[person]"));
    assert!(svg.contains("[account-login]"));
}

#[test]
fn salt_layout_depth_fixture_has_widget_dom_and_span_geometry() {
    let src = fs::read_to_string(fixture("families/valid_salt_layout_depth.puml"))
        .expect("fixture should load");
    let svg = render_source_to_svg(&src).expect("salt layout depth fixture should render");

    assert!(svg.contains("data-salt-style=\"canvas\""));
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#334155\""));
    assert_eq!(
        svg_elements_with_attr(&svg, "data-salt-widget", "groupbox").len(),
        2,
        "nested groupbox rows should render as distinct widgets"
    );
    assert!(svg.contains("data-salt-widget=\"menu\" data-salt-open=\"true\""));
    assert_eq!(
        svg_elements_with_attr(&svg, "data-salt-widget", "tab").len(),
        3
    );
    assert!(svg.contains("data-salt-tree-depth=\"0\""));
    assert!(svg.contains("data-salt-tree-depth=\"1\""));
    assert!(svg.contains("data-salt-sprite=\"folder\""));
    assert!(svg.contains("data-salt-sprite-ref=\"folder\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"account-login\""));
    assert!(svg.contains("data-salt-creole=\"true\""));

    let span = svg_elements_with_attr(&svg, "data-salt-colspan", "2")
        .into_iter()
        .next()
        .expect("fixture should render a merged table cell");
    assert!(
        span.contains("data-salt-span-width="),
        "span group should expose merged geometry"
    );

    let headers = svg_elements_with_attr(&svg, "data-salt-widget", "header");
    assert_eq!(headers.len(), 3);
    let field_header_x = svg_attr_i32_required(headers[0], "x");
    let value_header_x = svg_attr_i32_required(headers[1], "x");
    let notes_header_x = svg_attr_i32_required(headers[2], "x");
    assert!(
        field_header_x < value_header_x && value_header_x < notes_header_x,
        "table header x positions should increase left-to-right"
    );

    let tree_nodes = svg_elements_with_attr(&svg, "data-salt-widget", "tree");
    assert_eq!(tree_nodes.len(), 2);
    let root_tree = svg_group_with_attr(&svg, "data-salt-tree-depth", "0");
    let nested_tree = svg_group_with_attr(&svg, "data-salt-tree-depth", "1");
    let root_branch_x = svg_attr_i32_required(root_tree, "x1");
    let nested_branch_x = svg_attr_i32_required(nested_tree, "x1");
    assert!(
        nested_branch_x > root_branch_x,
        "nested tree item should be indented in geometry"
    );

    let textareas = svg_elements_with_attr(&svg, "data-salt-widget", "textarea");
    assert_eq!(textareas.len(), 2);
    assert!(textareas
        .iter()
        .any(|el| el.contains("data-salt-scroll-vertical=\"true\"")));
}

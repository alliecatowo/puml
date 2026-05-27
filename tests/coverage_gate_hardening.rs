use std::fs;

use puml::sprites::{
    bootstrap_icon_sprite, bootstrap_icon_sprites, material_icon_sprite, material_icon_sprites,
    openiconic_icon_names, openiconic_sprite, openiconic_sprites, parse_openiconic_ref_at,
    parse_sprite_ref_at, SpriteKind,
};
use puml::stdlib::{format_stdlib_listing, inventory_from_root, stdlib_paths_json};
use puml::{
    extract_metadata, normalize_family, parse, CompatMode, FrontendSelection, ParsePipelineOptions,
};
use tempfile::tempdir;

fn metadata_for(src: &str) -> puml::DiagramMetadata {
    let document = parse(src).expect("source should parse");
    let model = normalize_family(document.clone()).expect("source should normalize");
    extract_metadata(&document, &model)
}

#[test]
fn metadata_counts_cover_specialized_and_structured_families() {
    let cases = [
        (
            r#"@startjson
{"users":[{"name":"Ada"},{"name":"Lin"}]}
@endjson
"#,
            "json",
            "nodes",
            6,
        ),
        (
            r#"@startyaml
root:
  child: café
@endyaml
"#,
            "yaml",
            "nodes",
            2,
        ),
        (
            r#"@startnwdiag
nwdiag {
  network dmz {
    web [address = "10.0.0.10"];
  }
  group edge {
    web;
  }
}
@endnwdiag
"#,
            "nwdiag",
            "networks",
            1,
        ),
        (
            r#"@startarchimate
title Arch
Business_Object(order, "Order")
Application_Component(api, "API")
Rel_Assignment(order, api, "uses")
@endarchimate
"#,
            "archimate",
            "relations",
            1,
        ),
        (
            r#"@startregex
title Pattern
^ab(c|d)+$
@endregex
"#,
            "regex",
            "patterns",
            1,
        ),
        (
            r#"@startebnf
expr = term , { ("+" | "-") , term } ;
@endebnf
"#,
            "ebnf",
            "rules",
            1,
        ),
        (
            r#"@startmath
title Formula
\frac{a}{b} + \sqrt{x}
@endmath
"#,
            "math",
            "body_bytes",
            22,
        ),
        (
            r#"@startsdl
state Ready
Ready -> Done : go
@endsdl
"#,
            "sdl",
            "transitions",
            1,
        ),
        (
            r#"@startditaa
title Box
+---+
| A |
+---+
@endditaa
"#,
            "ditaa",
            "body_bytes",
            15,
        ),
        (
            r#"@startchart
title Scores
Alice: 1
Bob: 2.5
@endchart
"#,
            "chart",
            "data_points",
            2,
        ),
    ];

    for (src, family, count_key, minimum) in cases {
        let metadata = metadata_for(src);
        assert_eq!(metadata.family, family, "family mismatch for {family}");
        let actual = metadata
            .counts
            .get(count_key)
            .copied()
            .unwrap_or_else(|| panic!("missing count key {count_key} for {family}"));
        assert!(
            actual >= minimum,
            "expected {family}.{count_key} >= {minimum}, got {actual}"
        );
    }
}

#[test]
fn metadata_preserves_sequence_pages_themes_skinparams_and_warning_codes() {
    let document = parse(
        r#"@startuml
!theme plain
skinparam FooBar definitely-unsupported
skinparam sequenceMessageAlign center
title First Page
Alice -> Bob : hello
newpage Second Page
Bob -> Alice : done
@enduml
"#,
    )
    .expect("sequence with metadata should parse");
    let model = normalize_family(document.clone()).expect("sequence should normalize");
    let metadata = extract_metadata(&document, &model);

    assert_eq!(metadata.family, "sequence");
    assert_eq!(metadata.title.as_deref(), Some("First Page"));
    assert_eq!(metadata.counts.get("pages"), Some(&2));
    assert_eq!(metadata.counts.get("messages"), Some(&2));
    assert_eq!(metadata.themes, vec!["plain"]);
    assert!(metadata
        .skinparams
        .iter()
        .any(|skinparam| skinparam.key == "sequenceMessageAlign"));
    assert!(metadata
        .warnings
        .iter()
        .any(|warning| warning.code.as_deref() == Some("W_SKINPARAM_UNSUPPORTED")));
    assert_eq!(metadata.pages[0].title.as_deref(), Some("First Page"));
    assert_eq!(metadata.pages[1].title.as_deref(), Some("Second Page"));
}

#[test]
fn stdlib_inventory_formats_sorted_entries_aliases_and_json() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("awslib14/Compute")).expect("awslib dirs");
    fs::create_dir_all(root.join("C4")).expect("c4 dirs");
    fs::write(root.join("awslib14/Compute/EC2.puml"), "' ec2\n").expect("ec2");
    fs::write(root.join("C4/C4_Context.puml"), "' c4\n").expect("c4");
    fs::write(root.join("README.md"), "not a diagram\n").expect("readme");

    let entries = inventory_from_root(root).expect("inventory");
    let paths = entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    for expected in [
        "C4/C4_Context.puml",
        "awslib/Compute/EC2.puml",
        "awslib14/Compute/EC2.puml",
        "openiconic/account-login.puml",
        "openiconic/action-redo.puml",
    ] {
        assert!(
            paths.contains(&expected),
            "inventory missing expected stdlib path {expected}"
        );
    }
    let mut sorted_paths = paths.clone();
    sorted_paths.sort_unstable();
    assert_eq!(paths, sorted_paths, "stdlib inventory must be sorted");
    assert!(
        paths.contains(&"openiconic/all.puml"),
        "inventory should expose generated OpenIconic pack include"
    );
    assert!(
        paths.contains(&"openiconic/folder.puml"),
        "inventory should expose generated OpenIconic icon includes"
    );

    let alias = entries
        .iter()
        .find(|entry| entry.path == "awslib/Compute/EC2.puml")
        .expect("alias entry");
    assert!(alias.alias);
    assert_eq!(alias.physical_path, "awslib14/Compute/EC2.puml");

    let json = stdlib_paths_json(&entries);
    assert!(json.contains("C4/C4_Context.puml"));
    assert!(json.contains("awslib/Compute/EC2.puml"));
    assert!(json.contains("openiconic/folder.puml"));

    let listing = format_stdlib_listing(root, &entries);
    assert!(listing.contains("# alias: awslib -> awslib14"));
    assert!(listing.contains("awslib/Compute/EC2.puml -> awslib14/Compute/EC2.puml"));
    assert!(listing.contains("openiconic/folder.puml"));
    assert!(!listing.contains("README.md"));
}

#[test]
fn bundled_icon_resolvers_cover_aliases_inventory_and_missing_names() {
    let (angle_ref, consumed) = parse_openiconic_ref_at("<&folder,scale=2,color=#336699>!")
        .expect("OpenIconic angle reference");
    assert_eq!(angle_ref.name, "folder");
    assert_eq!(angle_ref.scale, 2.0);
    assert_eq!(angle_ref.color.as_deref(), Some("#336699"));
    assert_eq!(consumed, "<&folder,scale=2,color=#336699>".len());

    let (bare_ref, bare_consumed) =
        parse_openiconic_ref_at("&cloud_upload ok").expect("bare OpenIconic reference");
    assert_eq!(bare_ref.name, "cloud-upload");
    assert_eq!(bare_consumed, "&cloud_upload".len());
    assert!(parse_openiconic_ref_at("&not-a-real-icon").is_none());
    assert!(parse_sprite_ref_at("<$ma-cloud-upload{scale=1.5,color=#0f766e}>").is_some());

    let folder = openiconic_sprite("oi-folder").expect("folder icon");
    assert_eq!(folder.name, "folder");
    assert!(matches!(folder.kind, SpriteKind::Svg { .. }));
    assert!(openiconic_sprite("missing-folder").is_none());

    let bootstrap = bootstrap_icon_sprite("$bi_bootstrap_fill").expect("bootstrap icon alias");
    assert_eq!(bootstrap.name, "bi-bootstrap-fill");
    assert!(bootstrap_icon_sprite("bootstrap-fill").is_none());

    let material = material_icon_sprite("ma-cloud-upload").expect("material dash alias");
    assert_eq!(material.name, "ma_cloud_upload");
    assert!(material_icon_sprite("cloud_upload").is_none());

    let oi_registry = openiconic_sprites();
    assert!(oi_registry.contains_key("folder"));
    assert!(oi_registry.contains_key("cloud-upload"));
    let oi_names = openiconic_icon_names();
    assert_eq!(oi_names.len(), oi_registry.len());
    assert!(oi_names.windows(2).all(|pair| pair[0] < pair[1]));

    let bootstrap_registry = bootstrap_icon_sprites();
    assert!(bootstrap_registry.contains_key("bi-globe"));
    assert!(bootstrap_registry.contains_key("bi-bootstrap-fill"));

    let material_registry = material_icon_sprites();
    assert!(material_registry.contains_key("ma_folder"));
    assert!(material_registry.contains_key("ma_cloud_upload"));
}

fn sequence_labels_with_options(src: &str, options: ParsePipelineOptions) -> Vec<String> {
    let document = puml::parse_with_pipeline_options(src, &options).expect("source should parse");
    document
        .statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            puml::ast::StatementKind::Message(message) => message.label.clone(),
            _ => None,
        })
        .collect()
}

fn pipeline_with_root(root: &std::path::Path) -> ParsePipelineOptions {
    ParsePipelineOptions {
        frontend: FrontendSelection::Plantuml,
        compat: CompatMode::Strict,
        include_root: Some(root.to_path_buf()),
        allow_url_includes: false,
        inject_vars: Default::default(),
    }
}

#[test]
fn preproc_angle_stdlib_includes_tags_aliases_and_include_once() {
    let dir = tempdir().expect("tempdir");
    let stdlib = dir.path().join("stdlib/Local");
    fs::create_dir_all(&stdlib).expect("stdlib dirs");
    fs::write(
        stdlib.join("Tagged.puml"),
        "Alice -> Bob : whole\n!startsub ONLY\nAlice -> Bob : tagged\n!endsub\n",
    )
    .expect("tagged stdlib");

    let src = "@startuml
!include <Local/Tagged>!ONLY
!include <Local/Tagged>!ONLY
@enduml";
    let labels = sequence_labels_with_options(src, pipeline_with_root(dir.path()));
    assert_eq!(
        labels,
        vec!["tagged"],
        "stdlib angle includes are include-once"
    );

    let whole = sequence_labels_with_options(
        "@startuml
!include <Local/Tagged>
@enduml",
        pipeline_with_root(dir.path()),
    );
    assert_eq!(whole, vec!["whole", "tagged"]);
}

#[test]
fn preproc_include_many_supports_glob_and_file_url_sources() {
    let dir = tempdir().expect("tempdir");
    let fragments = dir.path().join("frags");
    fs::create_dir_all(&fragments).expect("fragment dir");
    fs::write(fragments.join("b.puml"), "Alice -> Bob : second\n").expect("b");
    fs::write(fragments.join("a.puml"), "Alice -> Bob : first\n").expect("a");
    fs::write(fragments.join("skip.txt"), "Alice -> Bob : skipped\n").expect("skip");

    let labels = sequence_labels_with_options(
        "@startuml
!include_many frags/?.puml
@enduml",
        pipeline_with_root(dir.path()),
    );
    assert_eq!(
        labels,
        vec!["first", "second"],
        "glob expansion stays sorted"
    );

    let file_url = format!("file://{}", fragments.join("a.puml").display());
    let mut options = pipeline_with_root(dir.path());
    options.allow_url_includes = true;
    let labels = sequence_labels_with_options(
        &format!("@startuml\n!include_many {file_url}\n@enduml"),
        options,
    );
    assert_eq!(labels, vec!["first"]);
}

#[test]
fn preproc_import_and_include_error_contracts_are_stable() {
    let dir = tempdir().expect("tempdir");
    let mut missing_root_options = pipeline_with_root(dir.path());
    missing_root_options.include_root = None;

    let cases = [
        (
            "@startuml\n!include <>\n@enduml",
            pipeline_with_root(dir.path()),
            "E_INCLUDE_PATH_REQUIRED",
        ),
        (
            "@startuml\n!include <Local/Tagged> trailing\n@enduml",
            pipeline_with_root(dir.path()),
            "E_INCLUDE_INVALID_FORM",
        ),
        (
            "@startuml\n!include <Local/Tagged>!BAD TAG\n@enduml",
            pipeline_with_root(dir.path()),
            "E_INCLUDE_INVALID_FORM",
        ),
        (
            "@startuml\n!include_many /tmp/nope.puml\n@enduml",
            pipeline_with_root(dir.path()),
            "E_INCLUDE_ABSOLUTE_PATH",
        ),
        (
            "@startuml\n!include_many https://example.invalid/a.puml\n@enduml",
            pipeline_with_root(dir.path()),
            "E_INCLUDE_URL_DISABLED",
        ),
        (
            "@startuml\n!import\n@enduml",
            pipeline_with_root(dir.path()),
            "E_IMPORT_PATH_REQUIRED",
        ),
        (
            "@startuml\n!import <Local/Thing>!TAG\n@enduml",
            pipeline_with_root(dir.path()),
            "E_IMPORT_INVALID_FORM",
        ),
        (
            "@startuml\n!import /tmp/nope.puml\n@enduml",
            pipeline_with_root(dir.path()),
            "E_IMPORT_ABSOLUTE_PATH",
        ),
        (
            "@startuml\n!import https://example.invalid/a.puml\n@enduml",
            pipeline_with_root(dir.path()),
            "E_INCLUDE_URL_DISABLED",
        ),
        (
            "@startuml\n!import Local/Thing\n@enduml",
            missing_root_options,
            "E_IMPORT_ROOT_REQUIRED",
        ),
    ];

    for (src, options, code) in cases {
        let err = match puml::parse_with_pipeline_options(src, &options) {
            Ok(_) => panic!("expected {code} for {src}"),
            Err(err) => err,
        };
        assert!(
            err.message.contains(code),
            "expected {code}, got {}",
            err.message
        );
    }
}

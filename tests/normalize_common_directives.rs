use puml::model::{LegendHAlign, LegendVAlign, MetadataHAlign, NormalizedDocument, ScaleSpec};

fn parse(source: &str) -> puml::Document {
    puml::parse(source).expect("parse")
}

#[test]
fn sequence_common_directives_preserve_metadata_scale_and_warning_text() {
    let source = r#"@startuml
title Sequence Title
center header Sequence Header
right footer Sequence Footer
caption Sequence Caption
legend top left
Sequence Legend
end legend
scale max 800*600
skinparam TotallyUnsupportedKey maybe
Alice -> Bob : ping
@enduml
"#;

    let model = puml::normalize(parse(source)).expect("normalize sequence");

    assert_eq!(model.title.as_deref(), Some("Sequence Title"));
    assert_eq!(model.header.as_deref(), Some("Sequence Header"));
    assert_eq!(model.header_align, MetadataHAlign::Center);
    assert_eq!(model.footer.as_deref(), Some("Sequence Footer"));
    assert_eq!(model.footer_align, MetadataHAlign::Right);
    assert_eq!(model.caption.as_deref(), Some("Sequence Caption"));
    assert_eq!(model.legend.as_deref(), Some("Sequence Legend"));
    assert_eq!(model.legend_halign, LegendHAlign::Left);
    assert_eq!(model.legend_valign, LegendVAlign::Top);
    assert_eq!(
        model.scale,
        Some(ScaleSpec::MaxFixed {
            width: 800,
            height: 600,
        })
    );
    assert_eq!(model.warnings.len(), 1);
    assert_eq!(
        model.warnings[0].message,
        "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `TotallyUnsupportedKey`"
    );
}

#[test]
fn family_and_timeline_common_directives_keep_existing_legend_modes() {
    let family_source = r#"@startmindmap
title MindMap Title
header MindMap Header
legend top left
Raw Family Legend
end legend
* Root
** Child
@endmindmap
"#;
    let family = match puml::normalize_family(parse(family_source)).expect("normalize mindmap") {
        NormalizedDocument::Family(family) => family,
        other => panic!("expected family document, got {other:?}"),
    };

    assert_eq!(family.title.as_deref(), Some("MindMap Title"));
    assert_eq!(family.header.as_deref(), Some("MindMap Header"));
    assert_eq!(family.legend.as_deref(), Some("Raw Family Legend"));
    assert_eq!(family.legend_halign, LegendHAlign::Left);
    assert_eq!(family.legend_valign, LegendVAlign::Top);

    let timeline_source = r#"@startgantt
title Gantt Title
header Gantt Header
legend Timeline Legend
[Task] lasts 1 days
@endgantt
"#;
    let timeline = match puml::normalize_family(parse(timeline_source)).expect("normalize gantt") {
        NormalizedDocument::Timeline(timeline) => timeline,
        other => panic!("expected timeline document, got {other:?}"),
    };

    assert_eq!(timeline.title.as_deref(), Some("Gantt Title"));
    assert_eq!(timeline.header.as_deref(), Some("Gantt Header"));
    assert_eq!(timeline.legend.as_deref(), Some("Timeline Legend"));
}

use crate::DiagramFamily;

/// Central metadata for a diagram-family surface known to the parser,
/// renderer, docs corpus, or visual manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramFamilySpec {
    /// Stable lowercase identifier used by APIs and diagnostics.
    pub id: &'static str,
    /// Human-readable family name for docs and UI surfaces.
    pub display_name: &'static str,
    /// Public render API family, when this registry entry maps one-to-one to it.
    pub diagram_family: Option<DiagramFamily>,
    /// Dedicated `@start...` marker when the marker alone identifies the family.
    ///
    /// Families dispatched from a shared `@startuml` block keep this as `None`
    /// and set `shared_start_marker` instead.
    pub start_marker: Option<&'static str>,
    /// Dedicated `@end...` marker paired with `start_marker`.
    pub end_marker: Option<&'static str>,
    /// Additional start markers accepted for the same family.
    pub alternate_start_markers: &'static [&'static str],
    /// Additional end markers accepted for the same family.
    pub alternate_end_markers: &'static [&'static str],
    /// Shared envelope marker for families detected from block contents.
    pub shared_start_marker: Option<&'static str>,
    /// Shared closing marker for families detected from block contents.
    pub shared_end_marker: Option<&'static str>,
    /// Repository-relative examples/docs folder, when one exists.
    pub docs_folder: Option<&'static str>,
    /// Family id used by the generated visual examples manifest.
    pub visual_manifest_family_id: Option<&'static str>,
}

const UML_START: &str = "@startuml";
const UML_END: &str = "@enduml";

const fn uml_family(
    id: &'static str,
    display_name: &'static str,
    diagram_family: DiagramFamily,
    docs_folder: &'static str,
    visual_manifest_family_id: &'static str,
) -> DiagramFamilySpec {
    DiagramFamilySpec {
        id,
        display_name,
        diagram_family: Some(diagram_family),
        start_marker: None,
        end_marker: None,
        alternate_start_markers: &[],
        alternate_end_markers: &[],
        shared_start_marker: Some(UML_START),
        shared_end_marker: Some(UML_END),
        docs_folder: Some(docs_folder),
        visual_manifest_family_id: Some(visual_manifest_family_id),
    }
}

const fn dedicated_family(
    id: &'static str,
    display_name: &'static str,
    diagram_family: DiagramFamily,
    start_marker: &'static str,
    end_marker: &'static str,
    docs_folder: &'static str,
    visual_manifest_family_id: &'static str,
) -> DiagramFamilySpec {
    DiagramFamilySpec {
        id,
        display_name,
        diagram_family: Some(diagram_family),
        start_marker: Some(start_marker),
        end_marker: Some(end_marker),
        alternate_start_markers: &[],
        alternate_end_markers: &[],
        shared_start_marker: None,
        shared_end_marker: None,
        docs_folder: Some(docs_folder),
        visual_manifest_family_id: Some(visual_manifest_family_id),
    }
}

const DIAGRAM_FAMILY_SPECS: &[DiagramFamilySpec] = &[
    uml_family(
        "sequence",
        "Sequence",
        DiagramFamily::Sequence,
        "docs/examples/sequence",
        "sequence",
    ),
    uml_family(
        "class",
        "Class",
        DiagramFamily::Class,
        "docs/examples/class",
        "class",
    ),
    uml_family(
        "object",
        "Object",
        DiagramFamily::Object,
        "docs/examples/object",
        "object",
    ),
    uml_family(
        "usecase",
        "Use Case",
        DiagramFamily::UseCase,
        "docs/examples/usecase",
        "usecase",
    ),
    uml_family(
        "component",
        "Component",
        DiagramFamily::Component,
        "docs/examples/component",
        "component",
    ),
    uml_family(
        "deployment",
        "Deployment",
        DiagramFamily::Deployment,
        "docs/examples/deployment",
        "deployment",
    ),
    uml_family(
        "state",
        "State",
        DiagramFamily::State,
        "docs/examples/state",
        "state",
    ),
    uml_family(
        "activity",
        "Activity",
        DiagramFamily::Activity,
        "docs/examples/activity",
        "activity",
    ),
    uml_family(
        "timing",
        "Timing",
        DiagramFamily::Timing,
        "docs/examples/timing",
        "timing",
    ),
    dedicated_family(
        "salt",
        "Salt",
        DiagramFamily::Salt,
        "@startsalt",
        "@endsalt",
        "docs/examples/salt",
        "salt",
    ),
    dedicated_family(
        "mindmap",
        "Mind Map",
        DiagramFamily::MindMap,
        "@startmindmap",
        "@endmindmap",
        "docs/examples/mindmap",
        "mindmap",
    ),
    dedicated_family(
        "wbs",
        "WBS",
        DiagramFamily::Wbs,
        "@startwbs",
        "@endwbs",
        "docs/examples/wbs",
        "wbs",
    ),
    dedicated_family(
        "gantt",
        "Gantt",
        DiagramFamily::Gantt,
        "@startgantt",
        "@endgantt",
        "docs/examples/gantt",
        "gantt",
    ),
    dedicated_family(
        "chronology",
        "Chronology",
        DiagramFamily::Chronology,
        "@startchronology",
        "@endchronology",
        "docs/examples/chronology",
        "chronology",
    ),
    dedicated_family(
        "json",
        "JSON",
        DiagramFamily::Json,
        "@startjson",
        "@endjson",
        "docs/examples/json",
        "json",
    ),
    dedicated_family(
        "yaml",
        "YAML",
        DiagramFamily::Yaml,
        "@startyaml",
        "@endyaml",
        "docs/examples/yaml",
        "yaml",
    ),
    dedicated_family(
        "nwdiag",
        "Network",
        DiagramFamily::Nwdiag,
        "@startnwdiag",
        "@endnwdiag",
        "docs/examples/nwdiag",
        "nwdiag",
    ),
    dedicated_family(
        "archimate",
        "ArchiMate",
        DiagramFamily::Archimate,
        "@startarchimate",
        "@endarchimate",
        "docs/examples/archimate",
        "archimate",
    ),
    dedicated_family(
        "regex",
        "Regex",
        DiagramFamily::Regex,
        "@startregex",
        "@endregex",
        "docs/examples/regex",
        "regex",
    ),
    dedicated_family(
        "ebnf",
        "EBNF",
        DiagramFamily::Ebnf,
        "@startebnf",
        "@endebnf",
        "docs/examples/ebnf",
        "ebnf",
    ),
    DiagramFamilySpec {
        alternate_start_markers: &["@startlatex"],
        alternate_end_markers: &["@endlatex"],
        ..dedicated_family(
            "math",
            "Math",
            DiagramFamily::Math,
            "@startmath",
            "@endmath",
            "docs/examples/math",
            "math",
        )
    },
    dedicated_family(
        "sdl",
        "SDL",
        DiagramFamily::Sdl,
        "@startsdl",
        "@endsdl",
        "docs/examples/sdl",
        "sdl",
    ),
    dedicated_family(
        "ditaa",
        "Ditaa",
        DiagramFamily::Ditaa,
        "@startditaa",
        "@endditaa",
        "docs/examples/ditaa",
        "ditaa",
    ),
    dedicated_family(
        "chart",
        "Chart",
        DiagramFamily::Chart,
        "@startchart",
        "@endchart",
        "docs/examples/chart",
        "chart",
    ),
    dedicated_family(
        "chen",
        "Chen",
        DiagramFamily::Chen,
        "@startchen",
        "@endchen",
        "docs/examples/chen",
        "chen",
    ),
    DiagramFamilySpec {
        id: "c4",
        display_name: "C4",
        diagram_family: None,
        start_marker: None,
        end_marker: None,
        alternate_start_markers: &[],
        alternate_end_markers: &[],
        shared_start_marker: Some(UML_START),
        shared_end_marker: Some(UML_END),
        docs_folder: Some("docs/examples/c4"),
        visual_manifest_family_id: Some("c4"),
    },
];

/// Return all known diagram-family registry entries.
pub fn diagram_family_specs() -> &'static [DiagramFamilySpec] {
    DIAGRAM_FAMILY_SPECS
}

/// Find a diagram-family spec by its stable id.
pub fn diagram_family_spec_by_id(id: &str) -> Option<&'static DiagramFamilySpec> {
    diagram_family_specs()
        .iter()
        .find(|spec| spec.id.eq_ignore_ascii_case(id))
}

/// Find a diagram-family spec by a dedicated start marker.
///
/// Shared `@startuml` families intentionally do not match here because the
/// marker alone does not identify which family the block contains.
pub fn diagram_family_spec_by_start_marker(marker: &str) -> Option<&'static DiagramFamilySpec> {
    let marker = marker_token(marker)?;
    diagram_family_specs().iter().find(|spec| {
        marker_matches(spec.start_marker, marker)
            || marker_list_matches(spec.alternate_start_markers, marker)
    })
}

/// Find a diagram-family spec by a dedicated end marker.
///
/// Shared `@enduml` families intentionally do not match here because the
/// marker alone does not identify which family the block contains.
pub fn diagram_family_spec_by_end_marker(marker: &str) -> Option<&'static DiagramFamilySpec> {
    let marker = marker_token(marker)?;
    diagram_family_specs().iter().find(|spec| {
        marker_matches(spec.end_marker, marker)
            || marker_list_matches(spec.alternate_end_markers, marker)
    })
}

/// Find a diagram-family spec by any dedicated start or end marker.
pub fn diagram_family_spec_by_marker(marker: &str) -> Option<&'static DiagramFamilySpec> {
    diagram_family_spec_by_start_marker(marker)
        .or_else(|| diagram_family_spec_by_end_marker(marker))
}

fn marker_token(marker: &str) -> Option<&str> {
    marker.trim().split_ascii_whitespace().next()
}

fn marker_matches(expected: Option<&'static str>, actual: &str) -> bool {
    expected.is_some_and(|expected| expected.eq_ignore_ascii_case(actual))
}

fn marker_list_matches(expected: &[&'static str], actual: &str) -> bool {
    expected
        .iter()
        .any(|expected| expected.eq_ignore_ascii_case(actual))
}

#[cfg(test)]
mod tests {
    use super::{
        diagram_family_spec_by_id, diagram_family_spec_by_marker,
        diagram_family_spec_by_start_marker, diagram_family_specs,
    };
    use crate::DiagramFamily;
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn registry_ids_are_unique() {
        let mut seen = BTreeSet::new();
        for spec in diagram_family_specs() {
            assert!(seen.insert(spec.id), "duplicate family id `{}`", spec.id);
        }
    }

    #[test]
    fn dedicated_markers_are_unique() {
        let mut seen = BTreeSet::new();
        for spec in diagram_family_specs() {
            for marker in spec
                .start_marker
                .into_iter()
                .chain(spec.end_marker)
                .chain(spec.alternate_start_markers.iter().copied())
                .chain(spec.alternate_end_markers.iter().copied())
            {
                let marker = marker.to_ascii_lowercase();
                assert!(
                    seen.insert(marker.clone()),
                    "duplicate dedicated marker `{marker}`"
                );
            }
        }
    }

    #[test]
    fn shared_uml_markers_are_documented_but_not_dedicated() {
        for spec in diagram_family_specs() {
            if spec.shared_start_marker.is_some() || spec.shared_end_marker.is_some() {
                assert_eq!(spec.start_marker, None, "{} mixes marker modes", spec.id);
                assert_eq!(spec.end_marker, None, "{} mixes marker modes", spec.id);
                assert_eq!(spec.shared_start_marker, Some("@startuml"));
                assert_eq!(spec.shared_end_marker, Some("@enduml"));
            }
        }
        assert!(diagram_family_spec_by_start_marker("@startuml").is_none());
    }

    #[test]
    fn known_core_families_are_listed() {
        let expected = [
            ("sequence", Some(DiagramFamily::Sequence)),
            ("class", Some(DiagramFamily::Class)),
            ("object", Some(DiagramFamily::Object)),
            ("usecase", Some(DiagramFamily::UseCase)),
            ("component", Some(DiagramFamily::Component)),
            ("deployment", Some(DiagramFamily::Deployment)),
            ("state", Some(DiagramFamily::State)),
            ("activity", Some(DiagramFamily::Activity)),
            ("timing", Some(DiagramFamily::Timing)),
            ("salt", Some(DiagramFamily::Salt)),
            ("mindmap", Some(DiagramFamily::MindMap)),
            ("wbs", Some(DiagramFamily::Wbs)),
            ("gantt", Some(DiagramFamily::Gantt)),
            ("chronology", Some(DiagramFamily::Chronology)),
            ("json", Some(DiagramFamily::Json)),
            ("yaml", Some(DiagramFamily::Yaml)),
            ("nwdiag", Some(DiagramFamily::Nwdiag)),
            ("archimate", Some(DiagramFamily::Archimate)),
            ("regex", Some(DiagramFamily::Regex)),
            ("ebnf", Some(DiagramFamily::Ebnf)),
            ("math", Some(DiagramFamily::Math)),
            ("sdl", Some(DiagramFamily::Sdl)),
            ("ditaa", Some(DiagramFamily::Ditaa)),
            ("chart", Some(DiagramFamily::Chart)),
            ("chen", Some(DiagramFamily::Chen)),
            ("c4", None),
        ];

        for (id, family) in expected {
            let spec = diagram_family_spec_by_id(id).unwrap_or_else(|| {
                panic!("missing diagram family registry entry `{id}`");
            });
            assert_eq!(spec.diagram_family, family, "wrong family for `{id}`");
        }
    }

    #[test]
    fn dedicated_marker_lookup_finds_primary_and_alias_markers() {
        assert_eq!(
            diagram_family_spec_by_marker("@startchen").map(|spec| spec.id),
            Some("chen")
        );
        assert_eq!(
            diagram_family_spec_by_marker("@endchen trailing").map(|spec| spec.id),
            Some("chen")
        );
        assert_eq!(
            diagram_family_spec_by_marker("@STARTLATEX").map(|spec| spec.id),
            Some("math")
        );
        assert_eq!(
            diagram_family_spec_by_marker("@endlatex").map(|spec| spec.id),
            Some("math")
        );
    }

    #[test]
    fn docs_folders_exist_for_registered_entries() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for spec in diagram_family_specs() {
            if let Some(folder) = spec.docs_folder {
                assert!(
                    root.join(folder).is_dir(),
                    "registered docs folder for `{}` does not exist: {folder}",
                    spec.id
                );
            }
        }
    }
}

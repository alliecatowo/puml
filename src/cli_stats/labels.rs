use puml::ast::{
    ActivityStepKind, ComponentNodeKind, DiagramKind, ParticipantRole, TimingDeclKind,
};

pub(super) fn participant_role_label(role: ParticipantRole) -> &'static str {
    match role {
        ParticipantRole::Participant => "participant",
        ParticipantRole::Actor => "actor",
        ParticipantRole::Boundary => "boundary",
        ParticipantRole::Control => "control",
        ParticipantRole::Entity => "entity",
        ParticipantRole::Database => "database",
        ParticipantRole::Collections => "collections",
        ParticipantRole::Queue => "queue",
    }
}

pub(super) fn component_kind_label(kind: ComponentNodeKind) -> &'static str {
    match kind {
        ComponentNodeKind::Action => "action",
        ComponentNodeKind::Agent => "agent",
        ComponentNodeKind::Component => "component",
        ComponentNodeKind::Interface => "interface",
        ComponentNodeKind::Port => "port",
        ComponentNodeKind::Node => "node",
        ComponentNodeKind::Artifact => "artifact",
        ComponentNodeKind::Boundary => "boundary",
        ComponentNodeKind::Cloud => "cloud",
        ComponentNodeKind::Circle => "circle",
        ComponentNodeKind::Collections => "collections",
        ComponentNodeKind::Frame => "frame",
        ComponentNodeKind::Storage => "storage",
        ComponentNodeKind::Container => "container",
        ComponentNodeKind::Control => "control",
        ComponentNodeKind::Database => "database",
        ComponentNodeKind::Entity => "entity",
        ComponentNodeKind::Package => "package",
        ComponentNodeKind::Rectangle => "rectangle",
        ComponentNodeKind::Folder => "folder",
        ComponentNodeKind::File => "file",
        ComponentNodeKind::Card => "card",
        ComponentNodeKind::Actor => "actor",
        ComponentNodeKind::Hexagon => "hexagon",
        ComponentNodeKind::Label => "label",
        ComponentNodeKind::Person => "person",
        ComponentNodeKind::Process => "process",
        ComponentNodeKind::Queue => "queue",
        ComponentNodeKind::Stack => "stack",
        ComponentNodeKind::UseCase => "use-case",
    }
}

pub(super) fn activity_step_kind_label(kind: &ActivityStepKind) -> &'static str {
    match kind {
        ActivityStepKind::Start => "activity-start",
        ActivityStepKind::Stop => "activity-stop",
        ActivityStepKind::End => "activity-end",
        ActivityStepKind::Action => "activity-action",
        ActivityStepKind::Arrow => "activity-arrow",
        ActivityStepKind::Connector => "activity-connector",
        ActivityStepKind::Note => "activity-note",
        ActivityStepKind::Kill => "activity-kill",
        ActivityStepKind::Detach => "activity-detach",
        ActivityStepKind::IfStart => "activity-if",
        ActivityStepKind::Else => "activity-else",
        ActivityStepKind::EndIf => "activity-endif",
        ActivityStepKind::RepeatStart => "activity-repeat",
        ActivityStepKind::RepeatWhile => "activity-repeat-while",
        ActivityStepKind::WhileStart => "activity-while",
        ActivityStepKind::EndWhile => "activity-endwhile",
        ActivityStepKind::Fork => "activity-fork",
        ActivityStepKind::ForkAgain => "activity-fork-again",
        ActivityStepKind::EndFork => "activity-endfork",
        ActivityStepKind::PartitionStart => "activity-partition",
        ActivityStepKind::PartitionEnd => "activity-partition-end",
    }
}

pub(super) fn timing_kind_label(kind: TimingDeclKind) -> &'static str {
    match kind {
        TimingDeclKind::Concise => "timing-concise",
        TimingDeclKind::Robust => "timing-robust",
        TimingDeclKind::Clock => "timing-clock",
        TimingDeclKind::Binary => "timing-binary",
    }
}

pub(super) fn class_group_kind_label(kind: &str) -> &'static str {
    match kind {
        "namespace" => "namespace",
        "package" => "package",
        "together" => "together",
        _ => "group",
    }
}

pub(super) fn diagram_kind_label(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Stdlib => "stdlib",
        DiagramKind::Chen => "chen",
        DiagramKind::Board => "board",
        DiagramKind::Files => "files",
        DiagramKind::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participant_role_labels_cover_cli_schema() {
        let cases = [
            (ParticipantRole::Participant, "participant"),
            (ParticipantRole::Actor, "actor"),
            (ParticipantRole::Boundary, "boundary"),
            (ParticipantRole::Control, "control"),
            (ParticipantRole::Entity, "entity"),
            (ParticipantRole::Database, "database"),
            (ParticipantRole::Collections, "collections"),
            (ParticipantRole::Queue, "queue"),
        ];

        for (role, label) in cases {
            assert_eq!(participant_role_label(role), label);
        }
    }

    #[test]
    fn component_kind_labels_cover_cli_schema() {
        let cases = [
            (ComponentNodeKind::Action, "action"),
            (ComponentNodeKind::Agent, "agent"),
            (ComponentNodeKind::Component, "component"),
            (ComponentNodeKind::Interface, "interface"),
            (ComponentNodeKind::Port, "port"),
            (ComponentNodeKind::Node, "node"),
            (ComponentNodeKind::Artifact, "artifact"),
            (ComponentNodeKind::Boundary, "boundary"),
            (ComponentNodeKind::Cloud, "cloud"),
            (ComponentNodeKind::Circle, "circle"),
            (ComponentNodeKind::Collections, "collections"),
            (ComponentNodeKind::Frame, "frame"),
            (ComponentNodeKind::Storage, "storage"),
            (ComponentNodeKind::Container, "container"),
            (ComponentNodeKind::Control, "control"),
            (ComponentNodeKind::Database, "database"),
            (ComponentNodeKind::Entity, "entity"),
            (ComponentNodeKind::Package, "package"),
            (ComponentNodeKind::Rectangle, "rectangle"),
            (ComponentNodeKind::Folder, "folder"),
            (ComponentNodeKind::File, "file"),
            (ComponentNodeKind::Card, "card"),
            (ComponentNodeKind::Actor, "actor"),
            (ComponentNodeKind::Hexagon, "hexagon"),
            (ComponentNodeKind::Label, "label"),
            (ComponentNodeKind::Person, "person"),
            (ComponentNodeKind::Process, "process"),
            (ComponentNodeKind::Queue, "queue"),
            (ComponentNodeKind::Stack, "stack"),
            (ComponentNodeKind::UseCase, "use-case"),
        ];

        for (kind, label) in cases {
            assert_eq!(component_kind_label(kind), label);
        }
    }

    #[test]
    fn activity_and_timing_kind_labels_cover_cli_schema() {
        let activity_cases = [
            (ActivityStepKind::Start, "activity-start"),
            (ActivityStepKind::Stop, "activity-stop"),
            (ActivityStepKind::End, "activity-end"),
            (ActivityStepKind::Action, "activity-action"),
            (ActivityStepKind::Arrow, "activity-arrow"),
            (ActivityStepKind::Connector, "activity-connector"),
            (ActivityStepKind::Note, "activity-note"),
            (ActivityStepKind::Kill, "activity-kill"),
            (ActivityStepKind::Detach, "activity-detach"),
            (ActivityStepKind::IfStart, "activity-if"),
            (ActivityStepKind::Else, "activity-else"),
            (ActivityStepKind::EndIf, "activity-endif"),
            (ActivityStepKind::RepeatStart, "activity-repeat"),
            (ActivityStepKind::RepeatWhile, "activity-repeat-while"),
            (ActivityStepKind::WhileStart, "activity-while"),
            (ActivityStepKind::EndWhile, "activity-endwhile"),
            (ActivityStepKind::Fork, "activity-fork"),
            (ActivityStepKind::ForkAgain, "activity-fork-again"),
            (ActivityStepKind::EndFork, "activity-endfork"),
            (ActivityStepKind::PartitionStart, "activity-partition"),
            (ActivityStepKind::PartitionEnd, "activity-partition-end"),
        ];

        for (kind, label) in activity_cases {
            assert_eq!(activity_step_kind_label(&kind), label);
        }

        let timing_cases = [
            (TimingDeclKind::Concise, "timing-concise"),
            (TimingDeclKind::Robust, "timing-robust"),
            (TimingDeclKind::Clock, "timing-clock"),
            (TimingDeclKind::Binary, "timing-binary"),
        ];

        for (kind, label) in timing_cases {
            assert_eq!(timing_kind_label(kind), label);
        }
    }

    #[test]
    fn group_and_diagram_kind_labels_cover_cli_schema() {
        for (kind, label) in [
            ("namespace", "namespace"),
            ("package", "package"),
            ("together", "together"),
            ("rectangle", "group"),
        ] {
            assert_eq!(class_group_kind_label(kind), label);
        }

        let diagram_cases = [
            (DiagramKind::Sequence, "sequence"),
            (DiagramKind::Class, "class"),
            (DiagramKind::Object, "object"),
            (DiagramKind::UseCase, "usecase"),
            (DiagramKind::Salt, "salt"),
            (DiagramKind::MindMap, "mindmap"),
            (DiagramKind::Wbs, "wbs"),
            (DiagramKind::Gantt, "gantt"),
            (DiagramKind::Chronology, "chronology"),
            (DiagramKind::Component, "component"),
            (DiagramKind::Deployment, "deployment"),
            (DiagramKind::State, "state"),
            (DiagramKind::Activity, "activity"),
            (DiagramKind::Timing, "timing"),
            (DiagramKind::Json, "json"),
            (DiagramKind::Yaml, "yaml"),
            (DiagramKind::Nwdiag, "nwdiag"),
            (DiagramKind::Archimate, "archimate"),
            (DiagramKind::Regex, "regex"),
            (DiagramKind::Ebnf, "ebnf"),
            (DiagramKind::Math, "math"),
            (DiagramKind::Sdl, "sdl"),
            (DiagramKind::Ditaa, "ditaa"),
            (DiagramKind::Chart, "chart"),
            (DiagramKind::Stdlib, "stdlib"),
            (DiagramKind::Chen, "chen"),
            (DiagramKind::Board, "board"),
            (DiagramKind::Files, "files"),
            (DiagramKind::Unknown, "unknown"),
        ];

        for (kind, label) in diagram_cases {
            assert_eq!(diagram_kind_label(kind), label);
        }
    }
}

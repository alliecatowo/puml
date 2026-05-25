use super::completion_extra::{extra_completion_specs, resolve_extra_completion_item};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: &'static str,
    pub kind: CompletionItemKind,
    pub detail: &'static str,
    pub documentation: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionItemKind {
    Keyword,
    Operator,
    Snippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionList {
    pub is_incomplete: bool,
    pub items: Vec<CompletionItem>,
}

pub fn completion_items() -> CompletionList {
    let mut items = completion_specs().to_vec();
    for item in extra_completion_specs() {
        if !items.iter().any(|existing| existing.label == item.label) {
            items.push((*item).clone());
        }
    }
    CompletionList {
        is_incomplete: false,
        items,
    }
}

pub fn resolve_completion_item(label: &str) -> Option<CompletionItem> {
    completion_specs()
        .iter()
        .find(|entry| entry.label == label)
        .cloned()
        .or_else(|| resolve_extra_completion_item(label))
}

fn completion_specs() -> &'static [CompletionItem] {
    use CompletionItemKind::{Keyword, Operator, Snippet};
    &[
        CompletionItem {
            label: "@startuml",
            kind: Keyword,
            detail: "Directive",
            documentation: "Start a sequence diagram block.",
        },
        CompletionItem {
            label: "@enduml",
            kind: Keyword,
            detail: "Directive",
            documentation: "End a sequence diagram block.",
        },
        CompletionItem {
            label: "title",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram title.",
        },
        CompletionItem {
            label: "header",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram header.",
        },
        CompletionItem {
            label: "footer",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram footer.",
        },
        CompletionItem {
            label: "caption",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Set a diagram caption.",
        },
        CompletionItem {
            label: "legend",
            kind: Keyword,
            detail: "Metadata",
            documentation: "Start a legend block.",
        },
        CompletionItem {
            label: "participant",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a participant.",
        },
        CompletionItem {
            label: "actor",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare an actor participant.",
        },
        CompletionItem {
            label: "boundary",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a boundary participant.",
        },
        CompletionItem {
            label: "control",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a control participant.",
        },
        CompletionItem {
            label: "entity",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare an entity participant.",
        },
        CompletionItem {
            label: "database",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a database participant.",
        },
        CompletionItem {
            label: "collections",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a collections participant.",
        },
        CompletionItem {
            label: "queue",
            kind: Keyword,
            detail: "Participant",
            documentation: "Declare a queue participant.",
        },
        CompletionItem {
            label: "box",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a participant box group.",
        },
        CompletionItem {
            label: "end box",
            kind: Keyword,
            detail: "Group",
            documentation: "End a participant box group.",
        },
        CompletionItem {
            label: "note left of",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note to the left side of a target.",
        },
        CompletionItem {
            label: "note right of",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note to the right side of a target.",
        },
        CompletionItem {
            label: "note over",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note over one or more targets.",
        },
        CompletionItem {
            label: "note across",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a note across all participants.",
        },
        CompletionItem {
            label: "hnote over",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a hex note over one or more targets.",
        },
        CompletionItem {
            label: "rnote over",
            kind: Keyword,
            detail: "Note",
            documentation: "Attach a rectangle note over one or more targets.",
        },
        CompletionItem {
            label: "ref over",
            kind: Keyword,
            detail: "Reference",
            documentation: "Declare a reference block over participants.",
        },
        CompletionItem {
            label: "alt",
            kind: Keyword,
            detail: "Group",
            documentation: "Start an alt block.",
        },
        CompletionItem {
            label: "else",
            kind: Keyword,
            detail: "Group",
            documentation: "Start an alternate branch within alt/par.",
        },
        CompletionItem {
            label: "opt",
            kind: Keyword,
            detail: "Group",
            documentation: "Start an opt block.",
        },
        CompletionItem {
            label: "loop",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a loop block.",
        },
        CompletionItem {
            label: "par",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a parallel block.",
        },
        CompletionItem {
            label: "break",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a break block.",
        },
        CompletionItem {
            label: "critical",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a critical block.",
        },
        CompletionItem {
            label: "group",
            kind: Keyword,
            detail: "Group",
            documentation: "Start a generic group block.",
        },
        CompletionItem {
            label: "end",
            kind: Keyword,
            detail: "Group",
            documentation: "End the current group or note block.",
        },
        CompletionItem {
            label: "activate",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Activate a participant lifeline.",
        },
        CompletionItem {
            label: "deactivate",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Deactivate a participant lifeline.",
        },
        CompletionItem {
            label: "create",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Create a participant instance.",
        },
        CompletionItem {
            label: "destroy",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Destroy a participant instance.",
        },
        CompletionItem {
            label: "return",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Emit a return message.",
        },
        CompletionItem {
            label: "autoactivate on",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Enable auto-activation on messages.",
        },
        CompletionItem {
            label: "autoactivate off",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Disable auto-activation on messages.",
        },
        CompletionItem {
            label: "autonumber",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Enable automatic message numbering.",
        },
        CompletionItem {
            label: "autonumber stop",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Stop automatic message numbering.",
        },
        CompletionItem {
            label: "autonumber resume",
            kind: Keyword,
            detail: "Lifecycle",
            documentation: "Resume automatic message numbering.",
        },
        CompletionItem {
            label: "hide footbox",
            kind: Keyword,
            detail: "Style",
            documentation: "Hide participant footboxes.",
        },
        CompletionItem {
            label: "show footbox",
            kind: Keyword,
            detail: "Style",
            documentation: "Show participant footboxes.",
        },
        CompletionItem {
            label: "skinparam sequence {}",
            kind: Snippet,
            detail: "Style",
            documentation: "Insert a sequence skinparam block.",
        },
        CompletionItem {
            label: "!include",
            kind: Keyword,
            detail: "Preprocessor",
            documentation: "Include another source file.",
        },
        CompletionItem {
            label: "!define",
            kind: Keyword,
            detail: "Preprocessor",
            documentation: "Define a preprocessor macro.",
        },
        CompletionItem {
            label: "!undef",
            kind: Keyword,
            detail: "Preprocessor",
            documentation: "Undefine a preprocessor macro.",
        },
        CompletionItem {
            label: "newpage",
            kind: Keyword,
            detail: "Pagination",
            documentation: "Split output into a new page.",
        },
        CompletionItem {
            label: "class",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare a class node.",
        },
        CompletionItem {
            label: "interface",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare an interface node.",
        },
        CompletionItem {
            label: "enum",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare an enum node.",
        },
        CompletionItem {
            label: "abstract class",
            kind: Keyword,
            detail: "Class Diagram",
            documentation: "Declare an abstract class node.",
        },
        CompletionItem {
            label: "package",
            kind: Keyword,
            detail: "Family Diagram",
            documentation: "Group family diagram nodes under a package.",
        },
        CompletionItem {
            label: "namespace",
            kind: Keyword,
            detail: "Family Diagram",
            documentation: "Group family diagram nodes under a namespace.",
        },
        CompletionItem {
            label: "state",
            kind: Keyword,
            detail: "State Diagram",
            documentation: "Declare a state node.",
        },
        CompletionItem {
            label: "[*]",
            kind: Keyword,
            detail: "State Diagram",
            documentation: "State diagram start or end marker.",
        },
        CompletionItem {
            label: "start",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Start an activity diagram flow.",
        },
        CompletionItem {
            label: "stop",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Stop an activity diagram flow.",
        },
        CompletionItem {
            label: "if",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Start an activity branch.",
        },
        CompletionItem {
            label: "then",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "Mark the positive activity branch.",
        },
        CompletionItem {
            label: "endif",
            kind: Keyword,
            detail: "Activity Diagram",
            documentation: "End an activity branch.",
        },
        CompletionItem {
            label: "== divider ==",
            kind: Snippet,
            detail: "Structure",
            documentation: "Insert a divider row.",
        },
        CompletionItem {
            label: "... delay ...",
            kind: Snippet,
            detail: "Structure",
            documentation: "Insert a delay row.",
        },
        CompletionItem {
            label: "|||",
            kind: Keyword,
            detail: "Structure",
            documentation: "Insert a spacer row.",
        },
        CompletionItem {
            label: "->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Solid message arrow.",
        },
        CompletionItem {
            label: "-->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Dashed message arrow.",
        },
        CompletionItem {
            label: "<-",
            kind: Operator,
            detail: "Arrow",
            documentation: "Solid reverse message arrow.",
        },
        CompletionItem {
            label: "<--",
            kind: Operator,
            detail: "Arrow",
            documentation: "Dashed reverse message arrow.",
        },
        CompletionItem {
            label: "->>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head forward arrow.",
        },
        CompletionItem {
            label: "-->>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head dashed forward arrow.",
        },
        CompletionItem {
            label: "<<-",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head reverse arrow.",
        },
        CompletionItem {
            label: "<<--",
            kind: Operator,
            detail: "Arrow",
            documentation: "Open-head dashed reverse arrow.",
        },
        CompletionItem {
            label: "->x",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow to lost endpoint.",
        },
        CompletionItem {
            label: "x->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow from found endpoint.",
        },
        CompletionItem {
            label: "-x",
            kind: Operator,
            detail: "Arrow",
            documentation: "Endpoint loss marker in expanded forms.",
        },
        CompletionItem {
            label: "->o",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow to open endpoint.",
        },
        CompletionItem {
            label: "o->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow from open endpoint.",
        },
        CompletionItem {
            label: "<->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Bidirectional solid arrow.",
        },
        CompletionItem {
            label: "<-->",
            kind: Operator,
            detail: "Arrow",
            documentation: "Bidirectional dashed arrow.",
        },
        CompletionItem {
            label: "-[#color]>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward arrow with custom color.",
        },
        CompletionItem {
            label: "-[#color,dashed]>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward dashed arrow with custom color.",
        },
        CompletionItem {
            label: "-[#color,bold]>",
            kind: Operator,
            detail: "Arrow",
            documentation: "Forward bold arrow with custom color.",
        },
        CompletionItem {
            label: "++",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Activate target lifeline after this message.",
        },
        CompletionItem {
            label: "--",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Deactivate source lifeline after this message.",
        },
        CompletionItem {
            label: "**",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Create target participant from this message.",
        },
        CompletionItem {
            label: "!!",
            kind: Operator,
            detail: "Lifecycle Suffix",
            documentation: "Destroy target participant from this message.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_baseline_includes_family_and_arrow_items() {
        let labels = completion_items()
            .items
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();

        assert!(labels.contains(&"@startuml"));
        assert!(labels.contains(&"participant"));
        assert!(labels.contains(&"class"));
        assert!(labels.contains(&"state"));
        assert!(labels.contains(&"start"));
        for expected in ["fork", "!theme", "component", "ArrowColor"] {
            assert!(labels.contains(&expected), "missing {expected}");
        }
        assert!(labels.contains(&"-->>"));
    }
}

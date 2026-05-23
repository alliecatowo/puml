use puml::{render_source_to_text, render_source_to_texts, TextOutputMode};

fn text(source: &str) -> String {
    render_source_to_text(source, TextOutputMode::Txt).expect("text render")
}

#[test]
fn sequence_text_output_covers_event_and_metadata_variants() {
    let source = r#"@startuml
header Top
title Flow
actor Alice
queue Jobs
Alice -> Jobs: enqueue
autonumber 10
activate Jobs
group retry
...
== divider ==
|||
return ack
end
destroy Jobs
caption Bottom
legend
Key
endlegend
@enduml
"#;

    let rendered = text(source);
    assert!(rendered.contains("header: Top"));
    assert!(rendered.contains("title: Flow"));
    assert!(rendered.contains("actor Alice"));
    assert!(rendered.contains("queue Jobs"));
    assert!(rendered.contains("Alice -> Jobs: enqueue"));
    assert!(rendered.contains("autonumber 10"));
    assert!(rendered.contains("activate Jobs"));
    assert!(rendered.contains("group retry"));
    assert!(rendered.contains("spacer"));
    assert!(rendered.contains("separator divider"));
    assert!(rendered.contains("return Jobs -> Alice ack"));
    assert!(rendered.contains("destroy Jobs"));
    assert!(rendered.contains("caption: Bottom"));
    assert!(rendered.contains("legend: Key"));
}

#[test]
fn multipage_family_text_outputs_each_page() {
    let pages = render_source_to_texts(
        r#"@startuml
title First
class Alpha
newpage Second
class Beta
@enduml
"#,
        TextOutputMode::Utxt,
    )
    .expect("family text pages");

    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("title: First"));
    assert!(pages[0].contains("└─ Class Alpha"));
    assert!(pages[1].contains("title: Second"));
    assert!(pages[1].contains("└─ Class Beta"));
}

#[test]
fn state_text_output_covers_regions_actions_and_transitions() {
    let rendered = text(
        r#"@startuml
title Machine
state Running {
  [*] --> Idle
  state Idle
  Idle: entry / prepare
  --
  state Busy
}
Running --> [*] : done
@enduml
"#,
    );

    assert!(rendered.contains("title: Machine"));
    assert!(rendered.contains("state"));
    assert!(rendered.contains("Normal Running"));
    assert!(rendered.contains("region 1"));
    assert!(rendered.contains("transitions"));
    assert!(rendered.contains("Running -> [*]: done"));
}

#[test]
fn timeline_text_output_covers_gantt_and_chronology_sections() {
    let gantt = text(
        r#"@startgantt
Project starts 2026-01-01
[Design] lasts 3 days
[Build] starts at [Design]'s end
[Launch] happens at [Build]'s end
@endgantt
"#,
    );
    assert!(gantt.contains("Gantt"));
    assert!(gantt.contains("project starts 2026-01-01"));
    assert!(gantt.contains("tasks"));
    assert!(gantt.contains("milestones"));
    assert!(gantt.contains("constraints"));

    let chronology = text(
        r#"@startchronology
title Releases
Alpha happens on 2026-01-01
Beta happens on 2026-02-01
@endchronology
"#,
    );
    assert!(chronology.contains("title: Releases"));
    assert!(chronology.contains("Chronology"));
    assert!(chronology.contains("Alpha happens 2026-01-01"));
}

#[test]
fn structured_and_topology_text_outputs_cover_tree_modes() {
    let json = text(
        r#"@startjson
{"users":[{"name":"Ada"},{"name":"Lin"}]}
@endjson
"#,
    );
    assert!(json.contains("json"));
    assert!(json.contains("|- users"));

    let yaml = render_source_to_text(
        r#"@startyaml
root:
  child: café
@endyaml
"#,
        TextOutputMode::Utxt,
    )
    .expect("yaml text");
    assert!(yaml.contains("yaml"));
    assert!(yaml.contains("root: {...}"));
    assert!(yaml.contains("café"));

    let nwdiag = text(
        r#"@startnwdiag
nwdiag {
  network dmz {
    address = "10.0.0.0/24";
    web [address = "10.0.0.10"];
  }
  group edge {
    web;
  }
}
@endnwdiag
"#,
    );
    assert!(nwdiag.contains("nwdiag"));
    assert!(nwdiag.contains("network dmz"));
    assert!(nwdiag.contains("node web address=10.0.0.10"));
    assert!(nwdiag.contains("groups"));
}

#[test]
fn specialized_text_outputs_cover_token_formatters() {
    let archimate = text(
        r#"@startarchimate
title Arch
Business_Object(order, "Order")
Application_Component(api, "API")
Rel_Assignment(order, api, "uses")
@endarchimate
"#,
    );
    assert!(archimate.contains("archimate"));
    assert!(archimate.contains("business object Order"));
    assert!(archimate.contains("relations"));

    let regex = text(
        r#"@startregex
title Pattern
^ab(c|d)+$
@endregex
"#,
    );
    assert!(regex.contains("title: Pattern"));
    assert!(regex.contains("regex"));
    assert!(regex.contains("anchor ^"));
    assert!(regex.contains("repeat +"));

    let ebnf = text(
        r#"@startebnf
expr = term , { ("+" | "-") , term } ;
@endebnf
"#,
    );
    assert!(ebnf.contains("ebnf"));
    assert!(ebnf.contains("expr ::="));
    assert!(ebnf.contains("repetition"));

    let math = text(
        r#"@startmath
title Formula
\frac{a}{b} + \sqrt{x}
@endmath
"#,
    );
    assert!(math.contains("title: Formula"));
    assert!(math.contains("math"));
    assert!(math.contains("\\frac{a}{b} + \\sqrt{x}"));

    let sdl = text(
        r#"@startsdl
state Ready
Ready -> Done : go
@endsdl
"#,
    );
    assert!(sdl.contains("sdl"));
    assert!(sdl.contains("states"));
    assert!(sdl.contains("Ready -> Done go"));

    let ditaa = text(
        r#"@startditaa
title Box
+---+
| A |
+---+
@endditaa
"#,
    );
    assert!(ditaa.contains("title: Box"));
    assert!(ditaa.contains("ditaa"));
    assert!(ditaa.contains("+---+"));

    let chart = text(
        r#"@startchart
title Scores
caption Round 1
Alice: 1
Bob: 2.5
@endchart
"#,
    );
    assert!(chart.contains("title: Scores"));
    assert!(chart.contains("chart"));
    assert!(chart.contains("Alice = 1"));
    assert!(chart.contains("Bob = 2.5"));
    assert!(chart.contains("caption: Round 1"));
}

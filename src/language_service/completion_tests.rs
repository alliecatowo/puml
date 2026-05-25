use super::completion::completion_items;

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

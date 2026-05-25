use super::text::TextOutputMode;
use super::text_output::{finish_text, push_meta, text_value};
use crate::model::TimelineDocument;

pub(super) fn render_timeline_text(doc: &TimelineDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "header", doc.header.as_deref(), mode);
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!("{:?}", doc.kind));
    if let Some(start) = &doc.project_start {
        lines.push(format!("project starts {}", text_value(start, mode)));
    }
    if !doc.tasks.is_empty() {
        lines.push(format!("tasks ({})", doc.tasks.len()));
        for task in &doc.tasks {
            let critical = if task.is_critical { " critical" } else { "" };
            lines.push(format!(
                "  {} start={} duration={} workload={}{}",
                text_value(&task.name, mode),
                task.start_day,
                task.duration_days,
                task.workload_days,
                critical
            ));
        }
    }
    if !doc.milestones.is_empty() {
        lines.push(format!("milestones ({})", doc.milestones.len()));
        for milestone in &doc.milestones {
            let when = milestone
                .happens_on
                .as_deref()
                .map(|v| format!(" on {}", text_value(v, mode)))
                .unwrap_or_default();
            let critical = if milestone.is_critical {
                " critical"
            } else {
                ""
            };
            lines.push(format!(
                "  {}{}{}",
                text_value(&milestone.name, mode),
                when,
                critical
            ));
        }
    }
    if !doc.chronology_events.is_empty() {
        lines.push(format!("events ({})", doc.chronology_events.len()));
        for event in &doc.chronology_events {
            let range = event
                .end
                .as_deref()
                .map(|end| format!(" to {}", text_value(end, mode)))
                .unwrap_or_default();
            let color = event
                .color
                .as_deref()
                .map(|color| format!(" color={}", text_value(color, mode)))
                .unwrap_or_default();
            let bracket = if event.bracket { " bracket" } else { "" };
            lines.push(format!(
                "  {} happens {}{}{}{}",
                text_value(&event.subject, mode),
                text_value(&event.when, mode),
                range,
                color,
                bracket
            ));
        }
    }
    if !doc.constraints.is_empty() {
        lines.push(format!("constraints ({})", doc.constraints.len()));
        for c in &doc.constraints {
            lines.push(format!(
                "  {} {} {}",
                text_value(&c.subject, mode),
                text_value(&c.kind, mode),
                text_value(&c.target, mode)
            ));
        }
    }
    push_meta(&mut lines, "caption", doc.caption.as_deref(), mode);
    push_meta(&mut lines, "legend", doc.legend.as_deref(), mode);
    finish_text(lines)
}

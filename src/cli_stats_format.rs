use crate::cli_stats::Stats;

/// Format statistics as a human-readable summary.
#[allow(dead_code)]
pub fn format_human(stats: &Stats) -> String {
    let mut out = String::new();

    out.push_str(&format!("nodes:         {}\n", stats.node_count));
    out.push_str(&format!("edges:         {}\n", stats.edge_count));
    out.push_str(&format!("families:      {}\n", stats.families.join(", ")));
    out.push_str(&format!("max nesting:   {}\n", stats.max_nesting_depth));

    if stats.node_kinds.is_empty() {
        out.push_str("node kinds:    (none)\n");
    } else {
        out.push_str("node kinds:\n");
        let mut i = 0;
        for (k, v) in &stats.node_kinds {
            if i >= 7 {
                break;
            }
            out.push_str(&format!("  {k:<20} {v}\n"));
            i += 1;
        }
    }

    out
}

/// Format statistics as a JSON string.
pub fn format_json(stats: &Stats) -> String {
    serde_json::to_string_pretty(stats).unwrap_or_else(|_| "{}".to_string())
}

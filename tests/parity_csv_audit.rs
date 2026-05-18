use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                cur.push('"');
                i += 2;
                continue;
            }
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if ch == ',' && !in_quotes {
            out.push(cur.trim().to_string());
            cur.clear();
            i += 1;
            continue;
        }
        cur.push(ch);
        i += 1;
    }
    out.push(cur.trim().to_string());
    out
}

fn load_csv(rel: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let raw = fs::read_to_string(repo_path(rel)).expect("csv should exist");
    let mut lines = raw.lines().filter(|l| !l.trim().is_empty());
    let header = split_csv_line(lines.next().expect("csv header"));
    let rows = lines.map(split_csv_line).collect::<Vec<_>>();
    (header, rows)
}

fn markdown_table_rows(rel: &str) -> Vec<Vec<String>> {
    let raw = fs::read_to_string(repo_path(rel)).expect("markdown should exist");
    let mut rows = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        // Skip separator rows like |---|---|
        if trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ' ' || c == ':')
        {
            continue;
        }
        let cols = trimmed
            .trim_matches('|')
            .split('|')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();
        rows.push(cols);
    }
    rows
}

#[test]
fn parity_gap_csv_statuses_are_machine_readable_and_non_blank() {
    let allowed: BTreeSet<&str> = ["implemented", "partial", "missing"].into_iter().collect();
    for rel in [
        "docs/internal/parity/parity_gap_core.csv",
        "docs/internal/parity/parity_gap_nonuml.csv",
    ] {
        let (header, rows) = load_csv(rel);
        let status_idx = header
            .iter()
            .position(|h| h == "status")
            .unwrap_or_else(|| panic!("{rel}: missing `status` column"));
        for (idx, row) in rows.iter().enumerate() {
            assert_eq!(
                row.len(),
                header.len(),
                "{rel}: row {} has malformed column count",
                idx + 2
            );
            let status = row[status_idx].trim();
            assert!(
                !status.is_empty(),
                "{rel}: row {} has blank status",
                idx + 2
            );
            assert!(
                allowed.contains(status),
                "{rel}: row {} has malformed status `{status}`",
                idx + 2
            );
        }
    }
}

#[test]
fn parity_source_of_truth_markdown_statuses_are_machine_readable() {
    let allowed: BTreeSet<&str> = ["implemented", "partial", "missing"].into_iter().collect();
    let rows = markdown_table_rows("docs/internal/parity/plantuml_parity_source_of_truth.md");
    assert!(
        !rows.is_empty(),
        "source-of-truth table should not be empty"
    );

    // First encountered row is header in this parser output.
    let header = &rows[0];
    let status_idx = header
        .iter()
        .position(|h| h.eq_ignore_ascii_case("status"))
        .expect("markdown table must include status column");
    let ref_idx = header
        .iter()
        .position(|h| h.to_ascii_lowercase().contains("reference"))
        .expect("markdown table must include reference column");

    for (i, row) in rows.iter().enumerate().skip(1) {
        if row[status_idx].eq_ignore_ascii_case("status") {
            continue;
        }
        assert_eq!(
            row.len(),
            header.len(),
            "markdown row {} has malformed column count",
            i + 1
        );
        let status = row[status_idx].trim();
        assert!(
            !status.is_empty(),
            "markdown row {} has blank status",
            i + 1
        );
        assert!(
            allowed.contains(status),
            "markdown row {} has malformed status `{status}`",
            i + 1
        );
        let reference = row[ref_idx].trim();
        assert!(
            reference.starts_with("https://plantuml.com/"),
            "markdown row {} has non-PlantUML reference `{reference}`",
            i + 1
        );
    }
}

#[test]
fn parity_source_of_truth_contains_required_official_reference_pages() {
    let raw = fs::read_to_string(repo_path("docs/internal/parity/plantuml_parity_source_of_truth.md"))
        .expect("source-of-truth markdown");
    let required = [
        "https://plantuml.com/sequence-diagram",
        "https://plantuml.com/skinparam",
        "https://plantuml.com/preprocessing",
        "https://plantuml.com/preprocessing-json",
        "https://plantuml.com/timing-diagram",
        "https://plantuml.com/component-diagram",
        "https://plantuml.com/deployment-diagram",
        "https://plantuml.com/state-diagram",
        "https://plantuml.com/gantt-diagram",
        "https://plantuml.com/chronology-diagram",
        "https://plantuml.com/mindmap-diagram",
        "https://plantuml.com/wbs-diagram",
        "https://plantuml.com/salt",
        "https://plantuml.com/nwdiag",
        "https://plantuml.com/json",
        "https://plantuml.com/yaml",
        "https://plantuml.com/regex",
        "https://plantuml.com/ebnf",
        "https://plantuml.com/ascii-math",
        "https://plantuml.com/ditaa",
        "https://plantuml.com/chart-diagram",
    ];
    for url in required {
        assert!(
            raw.contains(url),
            "missing required official reference url: {url}"
        );
    }
}

#[test]
fn nonuml_missing_rows_do_not_contradict_fixture_backed_support() {
    let (header, rows) = load_csv("docs/internal/parity/parity_gap_nonuml.csv");
    let status_idx = header
        .iter()
        .position(|h| h == "status")
        .expect("status column");
    let family_idx = header
        .iter()
        .position(|h| h == "family")
        .expect("family column");
    let feature_idx = header
        .iter()
        .position(|h| h == "feature")
        .expect("feature column");
    let evidence_idx = header
        .iter()
        .position(|h| h == "evidence")
        .expect("evidence column");

    let fixture_backed_families = [
        "gantt+chronology",
        "mindmap+wbs",
        "salt",
        "archimate",
        "nwdiag/network",
        "json+yaml",
        "regex",
        "ebnf",
        "math",
        "sdl",
        "ditaa",
        "chart",
    ];

    let fixture_paths = [
        "tests/fixtures/timeline/valid_gantt_baseline.puml",
        "tests/fixtures/timeline/valid_chronology_baseline.puml",
        "tests/fixtures/non_sequence/valid_json.puml",
        "tests/fixtures/non_sequence/valid_yaml.puml",
        "tests/fixtures/non_sequence/valid_nwdiag.puml",
        "tests/fixtures/non_sequence/valid_archimate.puml",
        "tests/fixtures/non_sequence/valid_regex.puml",
        "tests/fixtures/non_sequence/valid_ebnf.puml",
        "tests/fixtures/non_sequence/valid_math.puml",
        "tests/fixtures/non_sequence/valid_sdl.puml",
        "tests/fixtures/non_sequence/valid_ditaa.puml",
        "tests/fixtures/non_sequence/valid_chart_bar.puml",
        "tests/fixtures/non_sequence/valid_chart_pie.puml",
        "tests/fixtures/families/valid_salt_bootstrap.puml",
        "tests/fixtures/non_sequence/invalid_mindmap_diagram.puml",
        "tests/fixtures/non_sequence/invalid_wbs_diagram.puml",
    ];

    for p in fixture_paths {
        assert!(repo_path(p).exists(), "fixture evidence path missing: {p}");
    }

    for (idx, row) in rows.iter().enumerate() {
        let family = row[family_idx].trim();
        let feature = row[feature_idx].trim().to_ascii_lowercase();
        let evidence = row[evidence_idx].trim().to_ascii_lowercase();
        let status = row[status_idx].trim();
        if status == "missing" && evidence.contains("tests/fixtures/") {
            panic!(
                "docs/internal/parity/parity_gap_nonuml.csv row {} is `missing` but cites fixture evidence",
                idx + 2
            );
        }
        let feature_allows_missing_with_family_fixtures =
            feature.contains("projection") || feature.contains("cross-diagram");
        if fixture_backed_families.contains(&family) && status == "missing" {
            if feature_allows_missing_with_family_fixtures {
                continue;
            }
            panic!(
                "docs/internal/parity/parity_gap_nonuml.csv row {} marks `{family}` as missing despite fixture-backed support",
                idx + 2
            );
        }
    }
}

fn collect_doc_example_sources(dir: &str, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(repo_path(dir)).unwrap_or_else(|e| panic!("{dir}: {e}")) {
        let path = entry.expect("example entry").path();
        if path.is_dir() {
            collect_doc_example_sources(
                path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .expect("example path should be under repo root")
                    .to_str()
                    .expect("example path should be UTF-8"),
                out,
            );
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) == Some("puml") {
            out.push(path);
        }
    }
}

#[test]
fn docs_examples_do_not_advertise_stale_unsupported_markers() {
    let stale_markers = ["not yet supported", "unsupported by this parser"];
    let mut sources = Vec::new();
    collect_doc_example_sources("docs/examples", &mut sources);

    for path in sources {
        let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let lower = raw.to_ascii_lowercase();
        for marker in stale_markers {
            assert!(
                !lower.contains(marker),
                "{} still advertises stale unsupported status marker `{marker}`",
                path.display()
            );
        }
    }
}

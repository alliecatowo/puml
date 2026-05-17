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

#[test]
fn parity_gap_csv_statuses_are_machine_readable_and_non_blank() {
    let allowed: BTreeSet<&str> = ["implemented", "partial", "missing"].into_iter().collect();
    for rel in [
        "docs/audits/parity_gap_core.csv",
        "docs/audits/parity_gap_nonuml.csv",
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
fn nonuml_missing_rows_do_not_contradict_fixture_backed_support() {
    let (header, rows) = load_csv("docs/audits/parity_gap_nonuml.csv");
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
                "docs/audits/parity_gap_nonuml.csv row {} is `missing` but cites fixture evidence",
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
                "docs/audits/parity_gap_nonuml.csv row {} marks `{family}` as missing despite fixture-backed support",
                idx + 2
            );
        }
    }
}

//! Smoke test for `--watch` mode.
//!
//! Spawns the binary in watch mode against a valid PlantUML fixture and
//! verifies it can start up without crashing immediately.
//! `.spawn().expect(...)` is the real assertion: if the binary is missing,
//! not executable, or exits immediately with a startup error, the test fails.

use std::fs;
use std::io::Write;

/// Helper: write a minimal valid PlantUML file to a temp path.
fn write_temp_puml(path: &std::path::Path) {
    let mut f = fs::File::create(path).expect("create temp puml");
    writeln!(f, "@startuml\nAlice -> Bob: hello\n@enduml").unwrap();
}

#[test]
fn watch_loop_starts_without_crashing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let puml_path = dir.path().join("watch_smoke.puml");
    write_temp_puml(&puml_path);

    // Spawn the binary directly in watch mode.
    // .spawn().expect(...) IS the assertion: spawn failure panics the test.
    // No wait, no kill — the OS reaps the orphaned child when the test
    // process exits.
    std::process::Command::new(env!("CARGO_BIN_EXE_puml"))
        .args(["--watch", puml_path.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to spawn puml --watch");
}

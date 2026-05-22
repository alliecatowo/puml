//! Smoke test for `--watch` mode.
//!
//! Spawns the binary in watch mode against a valid PlantUML fixture and
//! verifies it can start up without crashing immediately.
//! The child is killed and waited on before the test exits so the smoke test
//! does not leave a watch process behind.

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

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_puml"))
        .args(["--watch", puml_path.to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to spawn puml --watch");

    std::thread::sleep(std::time::Duration::from_millis(200));
    if let Some(status) = child.try_wait().expect("poll puml --watch child") {
        panic!("puml --watch exited during smoke window: {status}");
    }
    child.kill().expect("kill puml --watch child");
    child.wait().expect("wait for puml --watch child");
}

# Issue 94 Run Log

- 2026-05-15 19:35 PDT: Reproduced baseline behavior from the workpad: Gantt and Chronology inputs were accepted as generic/incorrect non-UML lines and lacked stable AST/model nodes.
- 2026-05-15 20:05 PDT: Re-read official PlantUML Gantt and Chronology references and narrowed this slice to bracketed task/event natural-language statements.
- 2026-05-15 20:06 PDT: Found existing partial local implementation; first compile gate failed on non-exhaustive Rust matches in `normalize.rs`, `render.rs`, and `lib.rs`.
- 2026-05-15 20:15 PDT: Added Gantt/Chronology exhaustive match handling, fixed bracketed Gantt dependency parsing, and added non-UML fixtures plus AST/model snapshot tests.
- 2026-05-15 20:18 PDT: `INSTA_UPDATE=always cargo test --test integration non_uml -- --nocapture` passed and wrote the new baseline snapshots.
- 2026-05-15 21:10 PDT: Revalidated current workspace: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --test integration non_uml -- --nocapture`, `cargo test --test coverage_edges -- --nocapture`, and `cargo test` all passed.
- 2026-05-15 21:10 PDT: PR #163 latest head was `7ea6566`; GitHub Actions check `fmt-clippy-test-coverage-quick` was still in progress during local revalidation.

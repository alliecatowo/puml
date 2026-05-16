# Issue 94 Run Log

- 2026-05-15 19:35 PDT: Reproduced baseline behavior from the workpad: Gantt and Chronology inputs were accepted as generic/incorrect non-UML lines and lacked stable AST/model nodes.
- 2026-05-15 20:05 PDT: Re-read official PlantUML Gantt and Chronology references and narrowed this slice to bracketed task/event natural-language statements.
- 2026-05-15 20:06 PDT: Found existing partial local implementation; first compile gate failed on non-exhaustive Rust matches in `normalize.rs`, `render.rs`, and `lib.rs`.
- 2026-05-15 20:15 PDT: Added Gantt/Chronology exhaustive match handling, fixed bracketed Gantt dependency parsing, and added non-UML fixtures plus AST/model snapshot tests.
- 2026-05-15 20:18 PDT: `INSTA_UPDATE=always cargo test --test integration non_uml -- --nocapture` passed and wrote the new baseline snapshots.

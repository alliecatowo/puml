# Your first diagram in 60 seconds

## Step 1: Install

```bash
cargo install puml --bin puml
```

→ See [install.md](install.md) for other install options.

## Step 2: Write a diagram

```bash
cat > hello.puml <<'EOF'
@startuml
Alice -> Bob: Hello
Bob --> Alice: Ack
@enduml
EOF
```

## Step 3: Render to SVG

```bash
puml hello.puml
```

This writes `hello.svg` next to the source file. Open it in any browser or SVG viewer.

## Step 4: Render to PNG

```bash
puml --format png hello.puml
```

Writes `hello.png`. Add `--dpi 192` for high-DPI / retina output.

## Step 5: Embed in Markdown

```markdown
![My diagram](hello.svg)
```

SVG renders inline in GitHub Markdown, most wikis, and static site generators. Commit both `hello.puml` and `hello.svg` — reviewers see the source; the SVG renders automatically.

## Validate without writing output

```bash
puml --check hello.puml
```

Exits 0 if the diagram is valid. Great for CI linting.

## Lint diagrams embedded in a Markdown file

```bash
puml --from-markdown --check notes.md
```

Finds all fenced ` ```puml ` blocks and validates them.

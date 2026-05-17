# Contributing to puml

## Setup

### One-time developer setup

```bash
# Clone + enter
git clone <your-fork-or-repo-url>
cd puml

# Install Rust toolchain dependencies (rustfmt, clippy, llvm-cov, etc.)
./scripts/setup.sh

# Install lefthook git hooks (optional but strongly recommended)
./scripts/install-hooks.sh
```

`lefthook` is a fast, language-agnostic git hook runner. Once installed, it
enforces two guards automatically:

| Event        | Command                                          | Purpose                          |
|--------------|--------------------------------------------------|----------------------------------|
| `pre-commit` | `cargo fmt --check`                              | Reject unformatted commits       |
| `pre-push`   | `cargo clippy --all-targets -- -D warnings`      | Reject clippy violations         |
| `pre-push`   | `cargo test --lib --quiet`                       | Reject broken unit tests         |

Hooks are **opt-in** — CI runners and automated agents do not need to install
them. Hooks are skipped automatically during rebase and merge commits.

To **uninstall** the hooks:

```bash
./scripts/install-hooks.sh --uninstall
```

### Installing lefthook

If `./scripts/install-hooks.sh` reports that lefthook is not installed, pick
any of the following:

```bash
# via cargo (works anywhere a Rust toolchain is present):
cargo install lefthook

# via the official install script (Linux/macOS):
curl -sSfL https://raw.githubusercontent.com/evilmartians/lefthook/master/install.sh | sh

# via brew:
brew install lefthook

# via snap (Ubuntu):
snap install lefthook
```

Then re-run `./scripts/install-hooks.sh`.

---

## Development workflow

```bash
# Fast local loop (fmt + clippy + test)
./scripts/dev.sh

# Full quality gate (fmt + clippy + test + coverage + release build + bench gates)
./scripts/check-all.sh

# Quick quality gate (skips coverage + release build)
./scripts/check-all.sh --quick
```

---

## CI

GitHub Actions is the **source of truth** for all quality gates. The PR gate
runs:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. Coverage gate (≥ 90 % line coverage)
5. `./scripts/check-all.sh --quick --skip-bench`
6. Docs examples drift gate

Passing CI is required before merge. Local hooks are a convenience layer, not
a substitute.

---

## For agents working on this repo

**Before opening a PR, always run the full pre-push check:**

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --quiet
```

Checklist before `gh pr create`:

- [ ] `cargo fmt` — no formatting violations (CI will reject unformatted code)
- [ ] `cargo clippy --all-targets -- -D warnings` — zero clippy warnings
- [ ] `cargo test --quiet` — all tests pass
- [ ] No changes to `src/` outside the stated task scope
- [ ] No changes to `.github/workflows/` unless the task explicitly requires it
- [ ] PR title follows the `type: description` convention
- [ ] PR body references the relevant issue (e.g., `Closes #NNN`)

Agents that install lefthook via `./scripts/install-hooks.sh` get the
`cargo fmt --check` guard on `git commit` and the clippy + lib-test guard on
`git push` automatically. Agents that do not install hooks must run the above
checklist manually.

---

## Code style

- All Rust code must be formatted with `rustfmt` (`cargo fmt`).
- Zero clippy warnings (`cargo clippy --all-targets -- -D warnings`).
- No `unwrap()` in production paths — use proper `Result`/`Option` handling.
- Keep `src/` deterministic: no `HashMap` iteration order dependencies,
  no floating-point output that varies by platform.

---

## Commit messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) style:

```
type: short imperative description

Optional longer body.

Closes #NNN
```

Common types: `feat`, `fix`, `chore`, `docs`, `test`, `refactor`, `perf`.

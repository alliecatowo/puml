# Branch Protection Contract

Issue: #90

This repository requires branch protection (or an equivalent active ruleset) on `main` with the following minimum policy:

- required status check context: `fmt-clippy-test-coverage-quick` (from `.github/workflows/pr-gate.yml`)
- pull request review required before merge (at least 1 approval)
- force pushes disabled on `main`
- branch deletion disabled on `main`

## Validation Command

Run:

```bash
./scripts/branch-protection.sh verify
```

The command exits non-zero if the required policy is missing.

## Apply Command

To attempt live enforcement via GitHub API:

```bash
./scripts/branch-protection.sh apply
```

`apply` writes branch protection using the GitHub REST API and then runs `verify`.

## Audited Fallback When API Writes Are Blocked

If `apply` fails due repository permission limits (common for non-admin tokens):

1. Run `./scripts/branch-protection.sh verify` and keep the failing output.
2. Add the output to PR notes and reference issue `#90`.
3. Ask a repository admin to apply equivalent protection/ruleset settings in GitHub.
4. Re-run `./scripts/branch-protection.sh verify` and include the passing result in the same PR thread.

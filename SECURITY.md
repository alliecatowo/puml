# Security Policy

## Supported Versions

PUML is in early development (pre-1.0). Security fixes land on main and are included in the next tagged release. Older tagged releases are not patched — please upgrade to the latest 0.x.y to receive fixes.

┌─────────────────────────┬────────────────────┐
│         Version         │     Supported      │
├─────────────────────────┼────────────────────┤
│ main (latest)           │ :white_check_mark: │
├─────────────────────────┼────────────────────┤
│ Latest tagged 0.x.y     │ :white_check_mark: │
├─────────────────────────┼────────────────────┤
│ Earlier tagged releases │ :x:                │
└─────────────────────────┴────────────────────┘

Once PUML reaches 1.0, this matrix will be updated to define a formal support window for the most recent minor version line.

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.

Instead, report them privately using GitHub's built-in s

1. Go to https://github.com/alliecatowo/puml/security/ad
2. Fill in a description, affected versions, and reproduction steps
3. Submit the draft advisory

If you cannot use GitHub Security Advisories, email me@allisons.dev with the subject line [puml security] <short description>. Please include:

- A description of the issue and its impact
- Affected version(s) / commit SHA
- A minimal reproduction (input .puml file, command, expected vs. actual behavior)
- Any proof-of-concept code or crash dumps
- Whether you have a suggested fix

What to expect

┌────────────────────────────────────────┬─────────────────────────────────────────────────────────────────────────────────────────┐
│                 Stage                  │              et SLA                                        │
├────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────┤
│ Acknowledgement of receipt             │ within 72 hou                                              │
├────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────┤
│ Initial triage and severity assessment │ within 7 days                                              │
├────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────┤
│ Status update cadence after triage     │ at least ever                                              │
├────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────┤
│ Coordinated disclosure window          │ typically 90 gotiable based on severity and fix complexity │
└────────────────────────────────────────┴─────────────────────────────────────────────────────────────────────────────────────────┘

If the vulnerability is accepted, you will receive:

- Confirmation of the severity rating (CVSS v3.1 base score)
- A coordinated disclosure plan with target fix and publ
- Credit in the security advisory and CHANGELOG.md (unless you prefer to remain anonymous)
- A CVE identifier where applicable

If the vulnerability is declined, you will receive a wribecause the behavior is by design, the threat model doesnot cover it, or the issue depends on already-compromised inputs (see "Out of scope" below). You are welcome to discuss the decision before
any public disclosure.

## Scope

### In scope:

- Memory safety issues in the Rust crates (puml, puml-ls
- Crashes, panics, or hangs on adversarial .puml input
- Path traversal or arbitrary file read via !include / p
- Server-side request forgery (SSRF) via URL includes
- Denial of service via unbounded recursion, expansion,
- Sandbox escapes in the WASM build
- Supply chain issues in the published artifacts (cargo )
- Credential or secret leakage in CLI output, diagnostics, or rendered SVG

### Out of scope:

- Vulnerabilities requiring a malicious local user with filesystem write access
- Theoretical issues without a working proof of concept
- Findings against forks or unsupported versions
- Self-inflicted issues from passing untrusted input witilar opt-in escape hatches
- Social engineering, physical attacks, or denial of service against github.com itself

## Safe harbor

Good-faith security research conducted under this policy is authorized. We will not pursue legal action against researchers who:

- Make a good-faith effort to avoid privacy violations, data destruction, or service interruption
- Only interact with their own accounts and test environ
- Give us a reasonable opportunity to address the issue before public disclosure
- Do not exploit the vulnerability beyond what is necess

Thank you for helping keep PUML and its users safe.

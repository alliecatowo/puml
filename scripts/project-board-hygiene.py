#!/usr/bin/env python3
"""Report Project #3 board and pull request hygiene findings.

The default mode is read-only. Use --apply-done only when the authenticated
GitHub token can edit the user-owned Projects v2 board.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


DEFAULT_REPO = "alliecatowo/puml"
DEFAULT_PROJECT_OWNER = "alliecatowo"
DEFAULT_PROJECT_NUMBER = "3"
DEFAULT_PROJECT_ID = "PVT_kwHOBdlpmc4BX1zk"
DEFAULT_STATUS_FIELD_ID = "PVTSSF_lAHOBdlpmc4BX1zkzhS_7BE"
DEFAULT_STATUS_DONE_ID = "c603c19d"

CLOSES_RE = re.compile(r"\bCloses\s+#\d+\b", re.IGNORECASE)
NO_CLOSE_RE = re.compile(r"\bDoes not close an issue\b", re.IGNORECASE)


@dataclass(frozen=True)
class Finding:
    kind: str
    number: int | None
    title: str
    url: str
    status: str
    reason: str
    item_id: str = ""

    def as_dict(self) -> dict[str, Any]:
        return {
            "kind": self.kind,
            "number": self.number,
            "title": self.title,
            "url": self.url,
            "status": self.status,
            "reason": self.reason,
            "item_id": self.item_id,
        }


def run_gh(args: list[str]) -> Any:
    try:
        proc = subprocess.run(
            ["gh", *args],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
    except FileNotFoundError:
        raise SystemExit("[board-hygiene] gh CLI not found")
    except subprocess.CalledProcessError as exc:
        stderr = exc.stderr.strip()
        raise SystemExit(f"[board-hygiene] gh {' '.join(args)} failed: {stderr}") from exc

    if not proc.stdout.strip():
        return {}
    return json.loads(proc.stdout)


def load_json_file(path: str | None) -> Any | None:
    if not path:
        return None
    with Path(path).open(encoding="utf-8") as fh:
        return json.load(fh)


def project_items(args: argparse.Namespace) -> list[dict[str, Any]]:
    fixture = load_json_file(args.project_items_json)
    if fixture is not None:
        return fixture.get("items", fixture)

    data = run_gh(
        [
            "project",
            "item-list",
            args.project_number,
            "--owner",
            args.project_owner,
            "--limit",
            str(args.limit),
            "--format",
            "json",
        ]
    )
    items = data.get("items", [])
    if not args.skip_state_enrichment:
        enrich_project_item_states(items, args.repo)
    return items


def open_prs(args: argparse.Namespace) -> list[dict[str, Any]]:
    fixture = load_json_file(args.open_prs_json)
    if fixture is not None:
        return fixture.get("pullRequests", fixture.get("prs", fixture))

    return run_gh(
        [
            "pr",
            "list",
            "--repo",
            args.repo,
            "--state",
            "open",
            "--limit",
            str(args.limit),
            "--json",
            "number,title,url,body,isDraft",
        ]
    )


def content_for_item(item: dict[str, Any]) -> dict[str, Any]:
    content = item.get("content")
    if isinstance(content, dict):
        return content
    return {}


def status_for_item(item: dict[str, Any]) -> str:
    status = item.get("status")
    if status is not None:
        return str(status).strip()

    for field_value in item.get("fieldValues", []) or []:
        field_name = str(field_value.get("fieldName", field_value.get("name", "")))
        if field_name.lower() == "status":
            value = field_value.get("name", field_value.get("value", ""))
            return str(value).strip()

    return ""


def item_type(content: dict[str, Any]) -> str:
    explicit = str(content.get("type", content.get("__typename", ""))).lower()
    if "pullrequest" in explicit or explicit == "pr":
        return "pr"
    if "issue" in explicit:
        return "issue"

    url = str(content.get("url", ""))
    if "/pull/" in url:
        return "pr"
    if "/issues/" in url:
        return "issue"
    return explicit


def repo_for_content(content: dict[str, Any], default_repo: str) -> str:
    repo = str(content.get("repository") or "")
    if repo and not repo.startswith("http"):
        return repo

    url = str(content.get("url") or repo)
    match = re.search(r"github\.com/([^/]+/[^/]+)/", url)
    return match.group(1) if match else default_repo


def enrich_project_item_states(items: list[dict[str, Any]], default_repo: str) -> None:
    for item in items:
        content = content_for_item(item)
        if not content or content.get("state"):
            continue
        if status_for_item(item).lower() == "done":
            continue

        number = content.get("number")
        if not number:
            continue

        repo = repo_for_content(content, default_repo)
        typ = item_type(content)
        try:
            if typ == "pr":
                data = run_gh(
                    [
                        "pr",
                        "view",
                        str(number),
                        "--repo",
                        repo,
                        "--json",
                        "state,mergedAt",
                    ]
                )
                content["state"] = data.get("state", "")
                content["mergedAt"] = data.get("mergedAt", "")
            elif typ == "issue":
                data = run_gh(
                    [
                        "issue",
                        "view",
                        str(number),
                        "--repo",
                        repo,
                        "--json",
                        "state",
                    ]
                )
                content["state"] = data.get("state", "")
        except SystemExit as exc:
            print(f"[board-hygiene] WARN: could not enrich {repo}#{number}: {exc}", file=sys.stderr)


def item_state(content: dict[str, Any]) -> str:
    return str(content.get("state", "")).upper()


def item_is_merged_pr(content: dict[str, Any]) -> bool:
    state = item_state(content)
    return bool(
        content.get("merged")
        or content.get("mergedAt")
        or content.get("merged_at")
        or state == "MERGED"
    )


def item_is_closed_issue(content: dict[str, Any]) -> bool:
    return item_type(content) == "issue" and item_state(content) == "CLOSED"


def item_is_active(content: dict[str, Any]) -> bool:
    typ = item_type(content)
    state = item_state(content)
    if typ == "issue":
        return state == "OPEN"
    if typ == "pr":
        return state == "OPEN" and not item_is_merged_pr(content)
    return False


def normalize_number(value: Any) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def collect_board_findings(items: list[dict[str, Any]]) -> list[Finding]:
    findings: list[Finding] = []
    for item in items:
        content = content_for_item(item)
        if not content:
            continue

        status = status_for_item(item)
        status_done = status.lower() == "done"
        url = str(content.get("url", ""))
        number = normalize_number(content.get("number"))
        title = str(content.get("title", ""))
        item_id = str(item.get("id", ""))
        typ = item_type(content)

        if not status_done and (item_is_closed_issue(content) or item_is_merged_pr(content)):
            label = "closed issue" if typ == "issue" else "merged PR"
            findings.append(
                Finding(
                    kind="stale_done_status",
                    number=number,
                    title=title,
                    url=url,
                    status=status,
                    reason=f"{label} is not Done",
                    item_id=item_id,
                )
            )
        elif not status and item_is_active(content):
            label = "open issue" if typ == "issue" else "open PR"
            findings.append(
                Finding(
                    kind="blank_active_status",
                    number=number,
                    title=title,
                    url=url,
                    status=status,
                    reason=f"{label} has no Status",
                    item_id=item_id,
                )
            )
    return findings


def collect_pr_findings(prs: list[dict[str, Any]]) -> list[Finding]:
    findings: list[Finding] = []
    for pr in prs:
        body = str(pr.get("body") or "")
        if CLOSES_RE.search(body) or NO_CLOSE_RE.search(body):
            continue
        findings.append(
            Finding(
                kind="missing_pr_issue_link",
                number=normalize_number(pr.get("number")),
                title=str(pr.get("title", "")),
                url=str(pr.get("url", "")),
                status="open",
                reason="open PR body is missing `Closes #...` or `Does not close an issue`",
            )
        )
    return findings


def apply_done(args: argparse.Namespace, findings: list[Finding]) -> int:
    editable = [f for f in findings if f.kind == "stale_done_status" and f.item_id]
    if not editable:
        return 0

    project_id = os.environ.get("PROJECT_ID", DEFAULT_PROJECT_ID)
    status_field_id = os.environ.get("STATUS_FIELD_ID", DEFAULT_STATUS_FIELD_ID)
    done_id = os.environ.get("STATUS_DONE_ID", DEFAULT_STATUS_DONE_ID)

    updated = 0
    for finding in editable:
        print(f"[board-hygiene] set Done: #{finding.number} {finding.url}", file=sys.stderr)
        if args.dry_run:
            updated += 1
            continue
        subprocess.run(
            [
                "gh",
                "project",
                "item-edit",
                "--id",
                finding.item_id,
                "--project-id",
                project_id,
                "--field-id",
                status_field_id,
                "--single-select-option-id",
                done_id,
            ],
            check=True,
        )
        updated += 1
    return updated


def print_text_report(board_findings: list[Finding], pr_findings: list[Finding]) -> None:
    def section(title: str, findings: list[Finding]) -> None:
        print(title)
        if not findings:
            print("  none")
            return
        for finding in findings:
            number = f"#{finding.number}" if finding.number is not None else "(no number)"
            status = finding.status or "(blank)"
            print(f"  - {number} [{finding.kind}] status={status}: {finding.title}")
            if finding.url:
                print(f"    {finding.url}")
            print(f"    {finding.reason}")

    section("Project #3 board findings", board_findings)
    section("Open PR issue-link findings", pr_findings)
    print(
        f"Summary: board={len(board_findings)} pr_issue_link={len(pr_findings)} "
        f"total={len(board_findings) + len(pr_findings)}"
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Report stale Project #3 items and open PR issue-link hygiene."
    )
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", DEFAULT_REPO))
    parser.add_argument("--project-owner", default=os.environ.get("PROJECT_OWNER", DEFAULT_PROJECT_OWNER))
    parser.add_argument("--project-number", default=os.environ.get("PROJECT_NUMBER", DEFAULT_PROJECT_NUMBER))
    parser.add_argument("--limit", type=int, default=1000)
    parser.add_argument("--project-items-json", help="Read project item JSON from a fixture file")
    parser.add_argument("--open-prs-json", help="Read open PR JSON from a fixture file")
    parser.add_argument(
        "--skip-state-enrichment",
        action="store_true",
        help="Do not call gh issue/pr view to enrich project item state",
    )
    parser.add_argument("--json", action="store_true", help="Print machine-readable findings")
    parser.add_argument(
        "--fail-on-findings",
        action="store_true",
        help="Exit 1 when any finding is present",
    )
    parser.add_argument(
        "--apply-done",
        action="store_true",
        help="Set stale closed issue / merged PR items to Done. Report-only by default.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        default=True,
        help="Preview mutations when used with --apply-done. This is the default.",
    )
    parser.add_argument(
        "--no-dry-run",
        dest="dry_run",
        action="store_false",
        help="Allow --apply-done to edit the project board.",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    board_findings = collect_board_findings(project_items(args))
    pr_findings = collect_pr_findings(open_prs(args))
    findings = board_findings + pr_findings

    if args.json:
        print(json.dumps([finding.as_dict() for finding in findings], indent=2))
    else:
        print_text_report(board_findings, pr_findings)

    if args.apply_done:
        updated = apply_done(args, board_findings)
        mode = "would update" if args.dry_run else "updated"
        print(f"[board-hygiene] {mode} {updated} stale Done item(s)", file=sys.stderr)

    return 1 if args.fail_on_findings and findings else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

import importlib.util
import pathlib
import sys
import unittest


SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "scripts" / "project-board-hygiene.py"
SPEC = importlib.util.spec_from_file_location("project_board_hygiene", SCRIPT)
project_board_hygiene = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = project_board_hygiene
SPEC.loader.exec_module(project_board_hygiene)


class ProjectBoardHygieneTest(unittest.TestCase):
    def test_board_findings_cover_stale_and_blank_active_items(self):
        items = [
            {
                "id": "PVTI_closed_issue",
                "status": "In Progress",
                "content": {
                    "type": "Issue",
                    "number": 12,
                    "title": "Closed issue",
                    "url": "https://github.com/alliecatowo/puml/issues/12",
                    "state": "CLOSED",
                },
            },
            {
                "id": "PVTI_merged_pr",
                "status": "Merging",
                "content": {
                    "type": "PullRequest",
                    "number": 13,
                    "title": "Merged PR",
                    "url": "https://github.com/alliecatowo/puml/pull/13",
                    "state": "MERGED",
                    "mergedAt": "2026-05-17T12:00:00Z",
                },
            },
            {
                "id": "PVTI_blank_open_issue",
                "status": "",
                "content": {
                    "type": "Issue",
                    "number": 14,
                    "title": "Open issue",
                    "url": "https://github.com/alliecatowo/puml/issues/14",
                    "state": "OPEN",
                },
            },
            {
                "id": "PVTI_done_closed",
                "status": "Done",
                "content": {
                    "type": "Issue",
                    "number": 15,
                    "title": "Already done",
                    "url": "https://github.com/alliecatowo/puml/issues/15",
                    "state": "CLOSED",
                },
            },
        ]

        findings = project_board_hygiene.collect_board_findings(items)

        self.assertEqual(
            [finding.kind for finding in findings],
            ["stale_done_status", "stale_done_status", "blank_active_status"],
        )
        self.assertEqual([finding.number for finding in findings], [12, 13, 14])

    def test_pr_findings_require_closes_or_explicit_no_issue_phrase(self):
        prs = [
            {"number": 20, "title": "Linked", "body": "Closes #12"},
            {"number": 21, "title": "No issue", "body": "Does not close an issue"},
            {"number": 22, "title": "Missing", "body": "Implements cleanup."},
            {"number": 23, "title": "Lowercase", "body": "closes #365"},
        ]

        findings = project_board_hygiene.collect_pr_findings(prs)

        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].number, 22)
        self.assertEqual(findings[0].kind, "missing_pr_issue_link")


if __name__ == "__main__":
    unittest.main()

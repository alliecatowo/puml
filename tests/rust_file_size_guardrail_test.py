import importlib.util
import io
import pathlib
import sys
import tempfile
import unittest
from contextlib import redirect_stdout


SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "scripts" / "check_rust_file_sizes.py"
SPEC = importlib.util.spec_from_file_location("check_rust_file_sizes", SCRIPT)
check_rust_file_sizes = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = check_rust_file_sizes
SPEC.loader.exec_module(check_rust_file_sizes)


class RustFileSizeGuardrailTest(unittest.TestCase):
    def test_reports_authored_files_and_allowlists_generated_icons(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / "src").mkdir()
            (root / "src" / "large.rs").write_text("fn large() {}\n" * 4, encoding="utf-8")
            (root / "src" / "small.rs").write_text("fn small() {}\n", encoding="utf-8")
            (root / "src" / "bootstrap_icons.rs").write_text(
                "// @generated\n" * 5,
                encoding="utf-8",
            )

            authored, allowlisted = check_rust_file_sizes.collect_over_limit(root, threshold=3)

            self.assertEqual([(entry.path, entry.lines) for entry in authored], [("src/large.rs", 4)])
            self.assertEqual(
                [(entry.path, entry.lines) for entry in allowlisted],
                [("src/bootstrap_icons.rs", 5)],
            )

    def test_default_mode_warns_but_does_not_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / "src").mkdir()
            (root / "src" / "large.rs").write_text("fn large() {}\n" * 4, encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = check_rust_file_sizes.main(["--root", str(root), "--threshold", "3"])

            self.assertEqual(exit_code, 0)
            self.assertIn("warning-only guardrail", stdout.getvalue())
            self.assertIn("src/large.rs", stdout.getvalue())

    def test_future_enforcement_mode_can_fail_on_violations(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / "src").mkdir()
            (root / "src" / "large.rs").write_text("fn large() {}\n" * 4, encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = check_rust_file_sizes.main(
                    ["--root", str(root), "--threshold", "3", "--fail-on-violations"]
                )

            self.assertEqual(exit_code, 1)


if __name__ == "__main__":
    unittest.main()

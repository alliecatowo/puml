import importlib.util
import io
import pathlib
import sys
import tempfile
import unittest
from contextlib import redirect_stdout


SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "scripts" / "check_renderer_boundaries.py"
SPEC = importlib.util.spec_from_file_location("check_renderer_boundaries", SCRIPT)
check_renderer_boundaries = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = check_renderer_boundaries
SPEC.loader.exec_module(check_renderer_boundaries)


def write(root: pathlib.Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class RendererBoundaryGuardTest(unittest.TestCase):
    def test_render_core_rejects_parser_and_model_dependencies(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            write(root, "src/render_core.rs", "use crate::parser;\nuse crate::model::Thing;\n")

            violations = check_renderer_boundaries.collect_violations(root)

            self.assertEqual([violation.rule for violation in violations], ["render-core-neutral", "render-core-neutral"])

    def test_legacy_svg_page_api_is_limited_to_public_adapter(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            write(root, "src/api/render.rs", "pub fn ok() { render_svg_pages_from_model(); }\n")
            write(root, "src/cli_run/output.rs", "pub fn leak() { render_svg_pages_from_model(); }\n")

            violations = check_renderer_boundaries.collect_violations(root)

            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "artifact-boundary")
            self.assertEqual(violations[0].path, "src/cli_run/output.rs")

    def test_direct_family_svg_calls_are_adapter_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            write(root, "src/api/render.rs", "pub fn ok() { render::render_state_svg(doc); }\n")
            write(root, "src/bin/lsp_adapter/render.rs", "pub fn leak() { render::render_state_svg(doc); }\n")

            violations = check_renderer_boundaries.collect_violations(root)

            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "svg-adapter-boundary")

    def test_artifact_literals_stay_behind_constructor_boundary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            write(root, "src/render/mod.rs", "let _ = RenderArtifact { svg: String::new() };\n")
            write(root, "src/api/render.rs", "let _ = RenderArtifact { svg: String::new() };\n")

            violations = check_renderer_boundaries.collect_violations(root)

            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "artifact-constructor-boundary")

    def test_enforced_mode_returns_failure_for_violations(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            write(root, "src/render_core.rs", "use crate::frontend;\n")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = check_renderer_boundaries.main(
                    ["--root", str(root), "--fail-on-violations"]
                )

            self.assertEqual(exit_code, 1)
            self.assertIn("enforced guard", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()

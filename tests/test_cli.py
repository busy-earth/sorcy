import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from sorcy import cli


class SorcyCliTests(unittest.TestCase):
    def test_parse_requirement_name(self) -> None:
        self.assertEqual(cli.parse_requirement_name("requests>=2.32"), "requests")
        self.assertEqual(cli.parse_requirement_name("pydantic[email]>=2"), "pydantic")
        self.assertEqual(cli.parse_requirement_name("  my_pkg ; python_version<'3.13'"), "my-pkg")

    def test_collect_dependency_names(self) -> None:
        data = {
            "project": {
                "dependencies": ["requests>=2", "numpy"],
                "optional-dependencies": {"dev": ["pytest>=8"]},
            },
            "dependency-groups": {"lint": ["ruff"]},
            "tool": {"poetry": {"dependencies": {"python": "^3.12", "httpx": "^0.28"}}},
        }
        names = cli.collect_dependency_names(data, include_optional=True, include_groups=True)
        self.assertEqual(names, ["httpx", "numpy", "pytest", "requests", "ruff"])

    def test_extract_github_repo(self) -> None:
        self.assertEqual(
            cli.extract_github_repo("git+https://github.com/pallets/flask.git"),
            "pallets/flask",
        )
        self.assertEqual(
            cli.extract_github_repo("git@github.com:psf/requests.git"),
            "psf/requests",
        )
        self.assertIsNone(cli.extract_github_repo("https://example.com/nope"))

    def test_resolve_github_repo(self) -> None:
        def fake_fetcher(_: str) -> dict:
            return {"info": {"project_urls": {"Source": "https://github.com/psf/requests"}}}

        repo, url = cli.resolve_github_repo("requests", fetcher=fake_fetcher)
        self.assertEqual(repo, "psf/requests")
        self.assertEqual(url, "https://github.com/psf/requests")

    def test_run_writes_markdown(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            project = Path(tmp)
            project.joinpath("pyproject.toml").write_text(
                textwrap.dedent(
                    """
                    [project]
                    name = "demo"
                    version = "0.1.0"
                    dependencies = ["requests>=2"]
                    """
                ).strip()
                + "\n",
                encoding="utf-8",
            )
            output = project / "deps.md"

            with mock.patch(
                "sorcy.cli.resolve_github_repo",
                return_value=("psf/requests", "https://github.com/psf/requests"),
            ):
                rc = cli.run(project, output, include_optional=True, include_groups=True)

            self.assertEqual(rc, 0)
            content = output.read_text(encoding="utf-8")
            self.assertIn("`requests`", content)
            self.assertIn("`psf/requests`", content)


if __name__ == "__main__":
    unittest.main()

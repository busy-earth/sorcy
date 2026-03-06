import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from sorcy import cli
from sorcy import source_resolver


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
            source_resolver.extract_github_repo("git+https://github.com/pallets/flask.git"),
            "pallets/flask",
        )
        self.assertEqual(
            source_resolver.extract_github_repo("git@github.com:psf/requests.git"),
            "psf/requests",
        )
        self.assertIsNone(source_resolver.extract_github_repo("https://example.com/nope"))

    def test_extract_source_repo_multiple_forges(self) -> None:
        self.assertEqual(
            source_resolver.extract_source_repo("git+https://github.com/pallets/flask.git"),
            ("github.com/pallets/flask", "https://github.com/pallets/flask"),
        )
        self.assertEqual(
            source_resolver.extract_source_repo("https://gitlab.com/pallets/flask/-/tree/main"),
            ("gitlab.com/pallets/flask", "https://gitlab.com/pallets/flask"),
        )
        self.assertEqual(
            source_resolver.extract_source_repo("ssh://git@bitbucket.org/team/repo.git"),
            ("bitbucket.org/team/repo", "https://bitbucket.org/team/repo"),
        )
        self.assertEqual(
            source_resolver.extract_source_repo("https://codeberg.org/org/project/src/branch/main"),
            ("codeberg.org/org/project", "https://codeberg.org/org/project"),
        )

    def test_extract_source_repo_unknown_host_behavior(self) -> None:
        self.assertIsNone(source_resolver.extract_source_repo("https://example.com/team/repo"))
        self.assertEqual(
            source_resolver.extract_source_repo(
                "https://git.example.org/group/project.git", allow_unlisted_host=True
            ),
            ("git.example.org/group/project", "https://git.example.org/group/project"),
        )

    def test_resolve_source_repo(self) -> None:
        def fake_fetcher(_: str) -> dict:
            return {"info": {"project_urls": {"Source": "https://gitlab.com/psf/requests"}}}

        repo, url = source_resolver.resolve_source_repo("requests", fetcher=fake_fetcher)
        self.assertEqual(repo, "gitlab.com/psf/requests")
        self.assertEqual(url, "https://gitlab.com/psf/requests")

    def test_resolve_source_repo_prefers_repository_labels(self) -> None:
        def fake_fetcher(_: str) -> dict:
            return {
                "info": {
                    "project_urls": {
                        "Documentation": "https://requests.readthedocs.io",
                        "Repository": "https://codeberg.org/org/requests",
                    }
                }
            }

        repo, url = source_resolver.resolve_source_repo("requests", fetcher=fake_fetcher)
        self.assertEqual(repo, "codeberg.org/org/requests")
        self.assertEqual(url, "https://codeberg.org/org/requests")

    def test_resolve_source_repo_skips_pypi_project_url(self) -> None:
        def fake_fetcher(_: str) -> dict:
            return {
                "info": {
                    "project_urls": {"Homepage": "https://example.com"},
                    "home_page": "https://docs.example.com",
                    "project_url": "https://pypi.org/project/requests/",
                }
            }

        repo, url = source_resolver.resolve_source_repo("requests", fetcher=fake_fetcher)
        self.assertIsNone(repo)
        self.assertIsNone(url)

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
                "sorcy.cli.resolve_source_repo",
                return_value=("gitlab.com/psf/requests", "https://gitlab.com/psf/requests"),
            ):
                rc = cli.run(project, output, include_optional=True, include_groups=True)

            self.assertEqual(rc, 0)
            content = output.read_text(encoding="utf-8")
            self.assertIn("`requests`", content)
            self.assertIn("`gitlab.com/psf/requests`", content)


if __name__ == "__main__":
    unittest.main()

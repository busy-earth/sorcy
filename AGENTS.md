# AGENTS.md

## Cursor Cloud specific instructions

This is a Python project ("sorcy") in its initial state — only a README, LICENSE, and `.gitignore` exist. No application code, dependency manifests, or services are present yet.

### Environment

- **Python 3.12** is available system-wide (`/usr/bin/python3`).
- **pip 24.0** is available system-wide.
- No virtual environment or dependency file (`requirements.txt`, `pyproject.toml`) exists yet. When one is added, the update script should be revised to install dependencies (e.g., `pip install -r requirements.txt` or `pip install -e ".[dev]"`).
- The `.gitignore` is configured for Python projects (covers `__pycache__`, `.venv`, common tooling caches, etc.).

### Running / Testing

- Install locally: `python3 -m pip install -e .`
- Run CLI: `python3 -m sorcy .`
- Run tests: `python3 -m unittest discover -s tests -v`

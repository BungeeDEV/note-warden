import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest

from release import check, prepare


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        files = {
            "Cargo.toml": '[package]\nname = "note-warden"\nversion = "1.1.0"\n\n[dependencies]\nexample = "1.1.0"\n',
            "Cargo.lock": 'version = 4\n\n[[package]]\nname = "example"\nversion = "1.1.0"\n\n[[package]]\nname = "note-warden"\nversion = "1.1.0"\n',
            "package.json": json.dumps({"version": "1.1.0"}),
            "package-lock.json": json.dumps({"version": "1.1.0", "packages": {"": {"version": "1.1.0"}, "node_modules/example": {"version": "1.1.0"}}}),
            "README.md": 'App. Version `1.1.0`.\n',
            "CHANGELOG.md": '## Unreleased\n\n### Fixed\n* Preserve user data.\n\n## 1.1.0\n\n* Initial installer.\n',
        }
        for name, content in files.items():
            (self.root / name).write_text(content, encoding="utf-8")

    def snapshot(self):
        return {p.name: p.read_bytes() for p in self.root.iterdir()}

    def test_prepare_moves_notes_and_synchronizes_all_versions(self):
        with contextlib.redirect_stdout(io.StringIO()):
            prepare(self.root, "1.2.0")
        self.assertEqual(check(self.root), "1.2.0")
        changelog = (self.root / "CHANGELOG.md").read_text(encoding="utf-8")
        self.assertTrue(changelog.startswith("## Unreleased\n\n## 1.2.0\n\n### Fixed"))
        self.assertIn('example = "1.1.0"', (self.root / "Cargo.toml").read_text())
        self.assertIn('name = "example"\nversion = "1.1.0"', (self.root / "Cargo.lock").read_text())
        npm = json.loads((self.root / "package-lock.json").read_text())
        self.assertEqual(npm["packages"]["node_modules/example"]["version"], "1.1.0")

    def test_rejects_invalid_or_non_increasing_versions_without_writing(self):
        before = self.snapshot()
        for version in ("1.1.0", "1.0.9", "v1.2.0", "1.2.0-beta.1", "01.2.0", "1.2", "1.2.0\n"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                prepare(self.root, version)
            self.assertEqual(self.snapshot(), before)

    def test_empty_unreleased_notes_do_not_modify_files(self):
        (self.root / "CHANGELOG.md").write_text('## Unreleased\n\n### Fixed\n\n## 1.1.0\n\n* Initial.\n')
        before = self.snapshot()
        with self.assertRaisesRegex(ValueError, "Describe the changes"):
            prepare(self.root, "1.2.0")
        self.assertEqual(self.snapshot(), before)

    def test_mismatched_versions_fail(self):
        for name in ("Cargo.lock", "package.json", "package-lock.json", "README.md"):
            path = self.root / name
            original = path.read_text()
            path.write_text(original.replace("1.1.0", "1.0.0"))
            with self.subTest(name=name), self.assertRaises(ValueError):
                check(self.root)
            path.write_text(original)

    def test_missing_or_duplicate_release_notes_fail(self):
        path = self.root / "CHANGELOG.md"
        for content in ('## 1.1.0\n\n### Fixed\n', '## 1.1.0\n* First\n\n## 1.1.0\n* Duplicate\n'):
            path.write_text(content)
            with self.subTest(content=content), self.assertRaises(ValueError):
                check(self.root)

    def test_cannot_overwrite_existing_future_release(self):
        with (self.root / "CHANGELOG.md").open("a") as stream:
            stream.write("\n## 1.2.0\n* Already prepared.\n")
        before = self.snapshot()
        with self.assertRaisesRegex(ValueError, "already exist"):
            prepare(self.root, "1.2.0")
        self.assertEqual(self.snapshot(), before)


if __name__ == "__main__":
    unittest.main()

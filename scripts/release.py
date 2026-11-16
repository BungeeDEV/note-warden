"""Validate release metadata or prepare a stable release without publishing it."""

import argparse
import json
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
STABLE = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


def check(root: Path) -> str:
    cargo = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    version = cargo["package"]["version"]
    if not STABLE.fullmatch(version):
        raise ValueError("Only stable x.y.z versions are supported by the Windows updater.")
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    own_packages = [p for p in lock["package"] if p["name"] == cargo["package"]["name"]]
    if len(own_packages) != 1 or own_packages[0]["version"] != version:
        raise ValueError("Cargo.lock does not match the Cargo.toml app version.")
    package = json.loads((root / "package.json").read_text(encoding="utf-8"))
    npm_lock = json.loads((root / "package-lock.json").read_text(encoding="utf-8"))
    if any(v != version for v in (
        package["version"], npm_lock["version"], npm_lock["packages"][""]["version"]
    )):
        raise ValueError("Cargo.toml, package.json and package-lock.json versions must match.")
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    headings = re.findall(r"^## (.+)$", changelog, re.MULTILINE)
    if len(headings) != len(set(headings)):
        raise ValueError("CHANGELOG.md contains duplicate release headings.")
    section = re.search(rf"^## {re.escape(version)}\n(.*?)(?=^## |\Z)", changelog, re.M | re.S)
    if not section or not re.search(r"^[*-] \S", section[1], re.M):
        raise ValueError(f"CHANGELOG.md needs release notes under '## {version}'.")
    readme = (root / "README.md").read_text(encoding="utf-8")
    if re.findall(r"Version `([^`]+)`", readme) != [version]:
        raise ValueError("README.md must show the current app version once as Version `x.y.z`.")
    return version


def prepare(root: Path, version: str) -> None:
    current = check(root)
    if not STABLE.fullmatch(version):
        raise ValueError("Use a stable x.y.z version (no v prefix or prerelease suffix).")
    if tuple(map(int, version.split("."))) <= tuple(map(int, current.split("."))):
        raise ValueError(f"The new version must be greater than {current}.")
    paths = ["Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "README.md", "CHANGELOG.md"]
    texts = {name: (root / name).read_text(encoding="utf-8") for name in paths}
    changelog = texts["CHANGELOG.md"]
    if re.search(rf"^## {re.escape(version)}$", changelog, re.M):
        raise ValueError(f"Release notes for {version} already exist.")
    unreleased = re.search(r"^## Unreleased\n(.*?)(?=^## |\Z)", changelog, re.M | re.S)
    if not unreleased or not re.search(r"^[*-] \S", unreleased[1], re.M):
        raise ValueError("Describe the changes under '## Unreleased' before preparing a release.")
    # Only the application's version changes; dependency versions remain byte-for-byte intact.
    texts["Cargo.toml"], count = re.subn(
        rf'(?m)^(version\s*=\s*)"{re.escape(current)}"', rf'\g<1>"{version}"',
        texts["Cargo.toml"], count=1,
    )
    if count != 1:
        raise ValueError("Cannot locate the application version in Cargo.toml.")
    texts["Cargo.lock"], count = re.subn(
        rf'(?m)^(name = "note-warden"\nversion = )"{re.escape(current)}"',
        rf'\g<1>"{version}"', texts["Cargo.lock"], count=1,
    )
    if count != 1:
        raise ValueError("Cannot locate the application version in Cargo.lock.")
    for name in ("package.json", "package-lock.json"):
        data = json.loads(texts[name])
        data["version"] = version
        if name == "package-lock.json":
            data["packages"][""]["version"] = version
        texts[name] = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    texts["README.md"] = texts["README.md"].replace(f"Version `{current}`", f"Version `{version}`")
    texts["CHANGELOG.md"] = (
        changelog[:unreleased.start()] + f"## Unreleased\n\n## {version}\n"
        + unreleased[1].rstrip() + "\n\n" + changelog[unreleased.end():]
    )
    for name, content in texts.items():
        (root / name).write_text(content, encoding="utf-8", newline="\n")
    check(root)
    print(f"Prepared {current} -> {version}. Review git diff, run ./scripts/check.ps1, then open a PR.")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check", help="Check versions, lockfiles, README and release notes")
    release = commands.add_parser("prepare", help="Move Unreleased notes to a new version; does not commit, tag or publish")
    release.add_argument("version")
    args = parser.parse_args()
    try:
        if args.command == "check":
            print(f"Release metadata OK: {check(ROOT)}")
        else:
            prepare(ROOT, args.version)
    except (ValueError, KeyError, OSError) as error:
        sys.exit(str(error))


if __name__ == "__main__":
    main()

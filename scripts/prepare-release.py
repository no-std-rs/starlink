#!/usr/bin/env python3
"""Align Cargo's workspace version and internal dependencies with a release tag."""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
CRATES = ("starlink-cli", "starlink-core", "starlink-proto")


def replace_one(path, pattern, replacement):
    original = path.read_text()
    updated, count = re.subn(pattern, replacement, original, count=1, flags=re.MULTILINE)
    if count != 1:
        raise SystemExit(f"expected one version field in {path}: found {count}")
    path.write_text(updated)


def main():
    if len(sys.argv) != 2 or not re.fullmatch(r"v?\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", sys.argv[1]):
        raise SystemExit("usage: prepare-release.py vMAJOR.MINOR.PATCH[-PRERELEASE]")
    version = sys.argv[1].removeprefix("v")
    recorded = (ROOT / "version.txt").read_text().strip()
    if recorded != version:
        raise SystemExit(f"release tag {version} differs from version.txt {recorded}")

    replace_one(ROOT / "Cargo.toml", r'(?<=\[workspace\.package\]\n)version = "[^"]+"', f'version = "{version}"')
    cli = ROOT / "crates/cli/Cargo.toml"
    for name in ("starlink-core", "starlink-proto"):
        replace_one(cli, rf'^{name} = \{{ version = "[^"]+", path = "\.\./{name}" \}}$',
                    f'{name} = {{ version = "{version}", path = "../{name}" }}')
    lock = ROOT / "Cargo.lock"
    content = lock.read_text()
    for name in CRATES:
        pattern = rf'(\[\[package\]\]\nname = "{name}"\n)version = "[^"]+"'
        content, count = re.subn(pattern, lambda match: f'{match.group(1)}version = "{version}"', content)
        if count != 1:
            raise SystemExit(f"expected one lock entry for {name}: found {count}")
    lock.write_text(content)


if __name__ == "__main__":
    main()

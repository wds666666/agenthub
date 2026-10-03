#!/usr/bin/env python3
"""Check document links and package metadata without changing versions."""
import argparse
import json
import os
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def check(release_tag=None, publish=False):
    package = json.loads((ROOT / "package.json").read_text())
    version = package["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Unsupported package version")
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
    tauri = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())
    if cargo["workspace"]["package"]["version"] != version or tauri["version"] != version:
        raise ValueError("Package, Cargo and Tauri versions must match")
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    for name in ("agenthub-cli", "agenthub-core", "agenthub-desktop"):
        entries = [entry for entry in lock["package"] if entry["name"] == name]
        if len(entries) != 1 or entries[0]["version"] != version:
            raise ValueError(f"Cargo.lock version mismatch: {name}")
    tag = release_tag or f"v{version}"
    if tag != f"v{version}":
        raise ValueError("Release tag must match the existing package version")
    if publish:
        notes = ROOT / "docs/releases" / f"{tag}.md"
        if not notes.is_file() or not notes.read_text().strip():
            raise ValueError("Publication requires reviewed release notes")

    documents = list(ROOT.glob("*.md"))
    for directory in ("docs", "skills"):
        documents.extend((ROOT / directory).rglob("*.md"))
    for document in documents:
        # Ignore fenced code examples when checking local Markdown links.
        text = re.sub(r"```.*?```", "", document.read_text(), flags=re.S)
        for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", text):
            target = target.strip().split(' "', 1)[0].strip("<>")
            url = urlsplit(target)
            if url.scheme or url.netloc or not url.path:
                continue
            path = (document.parent / unquote(url.path)).resolve()
            if not path.is_relative_to(ROOT) or not path.exists():
                raise ValueError(f"Broken local link in {document.relative_to(ROOT)}: {target}")
    premium = json.loads((ROOT / "premium-ui.json").read_text())
    for name in ("runtimeTokens", "designDocument", "uxContract"):
        path = (ROOT / premium[name]).resolve()
        if not path.is_relative_to(ROOT) or not path.is_file():
            raise ValueError(f"Missing design reference: {name}")
    return version, tag


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-tag", default="")
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--github-output", action="store_true")
    args = parser.parse_args()
    try:
        version, tag = check(args.release_tag, args.publish)
        if args.github_output:
            with open(os.environ["GITHUB_OUTPUT"], "a") as output:
                output.write(f"version={version}\ntag={tag}\n")
        print(f"Project checks passed; version remains {version}")
    except (ValueError, KeyError, OSError) as error:
        print(f"Project check failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

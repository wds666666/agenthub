#!/usr/bin/env python3
"""Create an optional manual-install Skill archive, never install it in any tool."""
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

repo = Path(__file__).resolve().parent.parent
source = repo / 'skills/agenthub-manager'
destination = repo / 'target/release/optional/agenthub-manager.zip'
destination.parent.mkdir(parents=True, exist_ok=True)
with ZipFile(destination, 'w', ZIP_DEFLATED) as archive:
    for path in sorted(source.rglob('*')):
        if path.is_file():
            archive.write(path, path.relative_to(source.parent))
print(destination)

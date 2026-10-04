"""Build an exact ownership list; never infer ownership from an installed folder."""
import hashlib
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent.parent
    entries = []
    for source in sorted((root / 'skills/agenthub-manager').rglob('*')):
        if source.is_file():
            entries.append({'path': source.relative_to(root).as_posix(), 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()})
    source = root / 'scripts/windows-cli.ps1'
    entries.append({'path': 'windows-cli.ps1', 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()})
    document = {'product': 'AgentHub', 'schemaVersion': 1, 'files': entries,
                'legacy': json.loads((root / 'scripts/windows-legacy-resources.json').read_text())['files']}
    destination = root / 'src-tauri/binaries/windows-resources.json'
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(document, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()

"""Build an exact ownership list; never infer ownership from an installed folder."""
import hashlib
import base64
from xml.sax.saxutils import escape
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent.parent
    entries = []
    source = root / 'scripts/windows-cli.ps1'
    entries.append({'path': 'windows-cli.ps1', 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()})
    document = {'product': 'AgentHub', 'schemaVersion': 1, 'files': entries,
                'legacy': json.loads((root / 'scripts/windows-legacy-resources.json').read_text())['files']}
    destination = root / 'src-tauri/binaries/windows-resources.json'
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(document, indent=2) + '\n', encoding='utf-8')
    legacy = json.dumps(document['legacy'], separators=(',', ':'))
    script = (root / 'scripts/msi-preserve-skill.ps1').read_text().replace('__LEGACY_JSON__', legacy.replace("'", "''"))
    encoded = base64.b64encode(script.encode('utf-16le')).decode()
    fragment = f'''<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi"><Fragment>
<CustomAction Id="AgentHubPreserveLegacySkill" Directory="SystemFolder" Execute="immediate" Return="check" Impersonate="yes" ExeCommand="powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -EncodedCommand {escape(encoded)}" />
<InstallExecuteSequence><Custom Action="AgentHubPreserveLegacySkill" Before="RemoveExistingProducts">Installed OR WIX_UPGRADE_DETECTED</Custom></InstallExecuteSequence>
</Fragment></Wix>
'''
    (destination.parent / 'msi-preserve.wxs').write_text(fragment, encoding='utf-8')



if __name__ == '__main__':
    main()

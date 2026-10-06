#!/usr/bin/env python3
"""Exercise real deb ownership in an isolated root without changing the host system."""
import io
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile


def run(*args):
    return subprocess.check_output(args, stderr=subprocess.PIPE)


def main():
    repo = Path(__file__).resolve().parent.parent
    version = json.loads((repo / 'package.json').read_text())['version']
    package = Path(sys.argv[1]) if len(sys.argv) > 1 else repo / f'target/release/bundle/deb/AgentHub_{version}_amd64.deb'
    package = package.resolve()
    control = tarfile.open(fileobj=io.BytesIO(run('dpkg-deb', '--ctrl-tarfile', str(package))))
    for name in control.getnames():
        if Path(name).name == 'preinst':
            assert control.extractfile(name).read() == (repo / 'scripts/deb-preinst.sh').read_bytes()
        elif Path(name).name in {'postinst', 'prerm', 'postrm', 'triggers'}:
            raise RuntimeError('Unknown maintainer script; never execute it against the developer system')
    package_name = run('dpkg-deb', '-f', str(package), 'Package').decode().strip()
    with tempfile.TemporaryDirectory(prefix='agenthub-deb-upgrade-') as directory:
        temp = Path(directory)
        root, old = temp / 'isolated-root', temp / 'previous-package'
        root.mkdir()
        (old / 'DEBIAN').mkdir(parents=True)
        (old / 'DEBIAN/control').write_text(f'Package: {package_name}\nVersion: 0.1.4\nArchitecture: amd64\nMaintainer: Test <test@example.com>\nDescription: isolated previous package\n')
        stale = Path('usr/share/agenthub/skills/agenthub-manager/references/obsolete.md')
        (old / stale).parent.mkdir(parents=True)
        (old / stale).write_text('obsolete package resource')
        edited = Path('usr/share/agenthub/skills/agenthub-manager/references/edited.md')
        (old / edited).write_text('original package text')
        owned = [stale, edited]
        (old / 'DEBIAN/md5sums').write_text(''.join(f'{hashlib.md5((old / path).read_bytes()).hexdigest()}  {path.as_posix()}\n' for path in owned))
        old_package = temp / 'previous.deb'
        run('dpkg-deb', '-b', str(old), str(old_package))
        kept = {
            Path('home/test/.agenthub/skills/keep/SKILL.md'): b'# Keep\n',
            Path('home/test/.agenthub/state/agenthub.db'): b'private state',
            Path('home/test/.agenthub/secrets/master.key'): b'private key',
            Path('home/test/.agenthub/.git/HEAD'): b'ref: refs/heads/agenthub\n',
            Path('usr/share/agenthub/skills/agenthub-manager/my-notes.txt'): b'unknown user file',
        }
        for relative, value in kept.items():
            (root / relative).parent.mkdir(parents=True, exist_ok=True)
            (root / relative).write_bytes(value)
        dpkg = ['dpkg', '--force-script-chrootless', '--force-not-root', '--force-depends', '--no-triggers', '--root=' + str(root), '--log=' + str(temp / 'dpkg.log')]
        run(*dpkg, '--install', str(old_package))
        (root / edited).write_text('user-edited resource')
        for deb in [package, package]:
            run(*dpkg, '--install', str(deb))
        assert (root / (edited.as_posix() + '.user-preserved')).read_text() == 'user-edited resource'
        assert not (root / stale).exists()
        for executable in ['agenthub', 'agenthub-desktop']:
            installed = (root / 'usr/bin' / executable).read_bytes()
            compiled = (repo / 'target/release' / executable).read_bytes()
            # Tauri writes the distribution marker only into the packaged desktop.
            if executable == 'agenthub-desktop':
                installed = installed.replace(b'TAURI_BUNDLE_TYPE_VAR_DEB', b'TAURI_BUNDLE_TYPE_VAR_UNK')
                compiled = compiled.replace(b'TAURI_BUNDLE_TYPE_VAR_DEB', b'TAURI_BUNDLE_TYPE_VAR_UNK')
            assert installed == compiled, executable
        assert not (root / 'usr/share/agenthub/skills/agenthub-manager/SKILL.md').exists()
        for relative, value in kept.items():
            assert (root / relative).read_bytes() == value
        run(*dpkg, '--remove', package_name)
        for relative, value in kept.items():
            assert (root / relative).read_bytes() == value
    print('PASS: Ubuntu replacement/reinstall/uninstall cleans obsolete owned files, preserves library/history/state/keys and user files; bundled UI/CLI match sources; management Skill is not installed')


if __name__ == '__main__':
    main()

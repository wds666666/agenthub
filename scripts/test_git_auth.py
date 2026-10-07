#!/usr/bin/env python3
"""Ubuntu integration check: real Git smart HTTPS + built-in credential helper.
All repositories, credentials and TLS keys live under TemporaryDirectory.
Usage: python3 scripts/test_git_auth.py [path/to/agenthub] [path/to/agenthub-desktop]
"""
import base64
import http.server
import json
import os
from pathlib import Path
import shutil
import ssl
import sqlite3
import subprocess
import sys
import tempfile
import threading
import urllib.parse


def main():
    cli = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/agenthub').resolve()
    git = shutil.which('git')
    with tempfile.TemporaryDirectory(prefix='agenthub-auth-test-') as directory:
        temporary = Path(directory)
        home = temporary / 'home'
        home.mkdir()
        server_root = temporary / 'server'
        server_root.mkdir()
        repo = server_root / 'repo.git'
        subprocess.run([git, 'init', '--bare', str(repo)], check=True, capture_output=True)
        subprocess.run([git, '-C', str(repo), 'config', 'http.receivepack', 'true'], check=True)
        key, cert = temporary / 'key.pem', temporary / 'cert.pem'
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(key), '-out', str(cert), '-days', '1', '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost'], check=True, capture_output=True)
        token = 'fixture-private-token+abc'
        authorization = 'Basic ' + base64.b64encode(('tester:' + token).encode()).decode()
        mode = {'read_only': False}

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                self.handle_git()

            def do_POST(self):
                self.handle_git()

            def handle_git(self):
                parsed = urllib.parse.urlsplit(self.path)
                if self.headers.get('Authorization') != authorization:
                    self.send_response(401)
                    self.send_header('WWW-Authenticate', 'Basic realm="isolated-git"')
                    self.send_header('Content-Length', '0')
                    self.end_headers()
                    return
                if mode['read_only'] and ('git-receive-pack' in self.path):
                    self.send_response(403)
                    self.send_header('Content-Length', '0')
                    self.end_headers()
                    return
                body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
                env = dict(os.environ, HOME=str(home), GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
                           GIT_PROJECT_ROOT=str(server_root), GIT_HTTP_EXPORT_ALL='1', PATH_INFO=parsed.path,
                           QUERY_STRING=parsed.query, REQUEST_METHOD=self.command,
                           CONTENT_TYPE=self.headers.get('Content-Type', ''), CONTENT_LENGTH=str(len(body)), REMOTE_USER='tester')
                data = subprocess.run([git, 'http-backend'], input=body, capture_output=True, env=env, check=True).stdout
                headers, payload = data.split(b'\r\n\r\n', 1)
                self.send_response(200)
                for header in headers.decode().splitlines():
                    name, value = header.split(':', 1)
                    if name.lower() != 'status':
                        self.send_header(name, value.strip())
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        tls.load_cert_chain(cert, key)
        server.socket = tls.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        url = f'https://localhost:{server.server_port}/repo.git'
        outputs = []

        def run(root, *args, secret=None, success=True):
            env = dict(os.environ, HOME=str(home), AGENTHUB_HOME=str(root), GIT_CONFIG_NOSYSTEM='1',
                       GIT_CONFIG_GLOBAL=os.devnull, GIT_SSL_CAINFO=str(cert))
            result = subprocess.run([str(cli), *args], input=secret, text=True, capture_output=True, env=env, timeout=60)
            outputs.append(result.stdout + result.stderr)
            assert (result.returncode == 0) == success, f'{args}: {result.stderr}'
            assert token not in outputs[-1] and authorization not in outputs[-1]
            return json.loads(result.stdout) if result.stdout.lstrip().startswith(('{', '[')) else result.stdout

        try:
            root = temporary / 'library'
            # First-run failure must not initialize or retain staging credentials.
            empty_restore = temporary / 'empty-restore'
            run(empty_restore, 'bootstrap', url, '--username', 'tester', secret=token, success=False)
            assert not run(empty_restore, 'doctor', '--json')['initialized']
            assert not list(temporary.glob('empty-restore-restore-*'))
            bad_auth = temporary / 'bad-auth-restore'
            run(bad_auth, 'bootstrap', url, '--username', 'tester', secret='wrong-fixture', success=False)
            assert not list(temporary.glob('bad-auth-restore-restore-*'))
            run(root, 'init', '--empty')
            skill = root / 'skills' / 'one'
            skill.mkdir()
            (skill / 'SKILL.md').write_text('# One\n')
            run(root, 'git', 'commit', '-m', 'first', '--name', 'Tester', '--email', 'test@example.com')
            subprocess.run([git, '-C', str(root), 'config', 'remote.agenthub.url', url], check=True)
            subprocess.run([git, '-C', str(root), 'config', 'agenthub.remoteBranch', 'agenthub'], check=True)
            subprocess.run([git, '-C', str(root), 'config', 'credential.username', 'unrelated-system-user'], check=True)
            assert run(root, 'git', 'remote-status')['state'] == 'unverified'
            run(root, 'git', 'login', url, '--branch', 'agenthub', '--username', 'tester', secret='wrong-fixture', success=False)
            assert not run(root, 'git', 'remote-status')['credential_saved']
            verified = run(root, 'git', 'login', url, '--branch', 'agenthub', '--username', 'tester', secret=token)
            assert verified['state'] == 'read_verified' and verified['credential_saved']
            if len(sys.argv) > 2:
                # Desktop helper dispatch must not initialize GTK or require DISPLAY.
                gui = Path(sys.argv[2]).resolve()
                env = dict(os.environ, HOME=str(home), AGENTHUB_GIT_ROOT=str(root))
                env.pop('DISPLAY', None)
                env.pop('WAYLAND_DISPLAY', None)
                request = f'protocol=https\nhost=localhost:{server.server_port}\npath=repo.git\n\n'
                helper = subprocess.run([str(gui), '__git-credential', 'get'], input=request, text=True, capture_output=True, env=env, timeout=10)
                assert helper.returncode == 0 and helper.stdout == f'username=tester\npassword={token}\n\n'
                assert not helper.stderr
            run(root, 'git', 'sync')
            assert run(root, 'git', 'remote-status')['state'] == 'synced'
            assert subprocess.run([git, '-C', str(repo), 'rev-parse', 'refs/heads/agenthub'], capture_output=True).returncode == 0
            # A second device merges separate resources instead of replacing the remote.
            second = temporary / 'second-library'
            run(second, 'init', '--empty')
            skill2 = second / 'skills' / 'two'
            skill2.mkdir()
            (skill2 / 'SKILL.md').write_text('# Two\n')
            run(second, 'git', 'commit', '-m', 'second', '--name', 'Tester', '--email', 'test@example.com')
            run(second, 'git', 'login', url, '--branch', 'agenthub', '--username', 'tester', secret=token)
            # Remote updates never project hosts, even with an enabled local profile.
            profile = {'target': 'agents', 'enabled': True, 'needs_review': False,
                       'selection': {'mode': 'preserve', 'skills_managed': True, 'skills': ['one'],
                                     'plugins': [], 'mcp': [], 'rule_ids': []}}
            with sqlite3.connect(second / 'state/agenthub.db') as database:
                database.execute('INSERT OR REPLACE INTO meta(key,value) VALUES (?,?)',
                                 ('auto_sync_profile_agents', json.dumps(profile)))
            host_skills = home / '.agents/skills'
            own = host_skills / 'host-only'
            own.mkdir(parents=True)
            (own / 'SKILL.md').write_text('# Host only\n')
            received = run(second, 'git', 'sync')
            assert received['auto_sync'] == [] and not received['auto_sync_error']
            assert (second / 'skills/one/SKILL.md').exists()
            assert not (host_skills / 'one').exists()
            projected = run(second, 'auto-sync', 'run')
            assert len(projected) == 1 and not projected[0].get('error')
            assert (host_skills / 'one/SKILL.md').read_text() == '# One\n'
            assert not (host_skills / 'two').exists()
            assert (own / 'SKILL.md').read_text() == '# Host only\n'
            (host_skills / 'one/SKILL.md').write_text('# Local improvement\n')
            unchanged = run(second, 'git', 'sync')
            assert unchanged['auto_sync'] == []
            assert (host_skills / 'one/SKILL.md').read_text() == '# Local improvement\n'
            # A dirty/staged local library can explicitly adopt a reviewed remote
            # snapshot without resetting history, pushing, or deploying tool content.
            before_head = subprocess.check_output([git, '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
            host_before = (host_skills / 'one/SKILL.md').read_text()
            (skill / 'SKILL.md').write_text('# Unsaved local\n')
            subprocess.run([git, '-C', str(root), 'add', 'skills/one'], check=True)
            local_only = root / 'skills/local-only'
            local_only.mkdir()
            (local_only / 'SKILL.md').write_text('# Local only\n')
            mode['read_only'] = True
            preview = run(root, 'git', 'recovery-plan', 'remote')
            assert any(item['id'] == 'local-only' and item['action'] == 'delete' for item in preview['changes'])
            run(root, 'git', 'recovery-apply', preview['id'], '--confirmation', 'DISCARD', success=False)
            remote_parent = subprocess.check_output([git, '-C', str(repo), 'rev-parse', 'refs/heads/agenthub'], text=True).strip()
            remote_tree = subprocess.check_output([git, '-C', str(repo), 'rev-parse', remote_parent + '^{tree}'], text=True).strip()
            raced = subprocess.check_output([git, '-C', str(repo), '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.com', 'commit-tree', remote_tree, '-p', remote_parent, '-m', 'cloud race'], text=True).strip()
            subprocess.run([git, '-C', str(repo), 'update-ref', 'refs/heads/agenthub', raced], check=True)
            run(root, 'git', 'recovery-apply', preview['id'], '--confirmation', 'REMOTE', success=False)
            assert (skill / 'SKILL.md').read_text() == '# Unsaved local\n'
            preview = run(root, 'git', 'recovery-plan', 'remote')
            recovery = run(root, 'git', 'recovery-apply', preview['id'], '--confirmation', 'REMOTE')
            assert (root / 'skills/one/SKILL.md').read_text() == '# One\n'
            assert (root / 'skills/two/SKILL.md').read_text() == '# Two\n'
            assert not local_only.exists()
            assert (Path(recovery['backup_path']) / 'skills/local-only/SKILL.md').read_text() == '# Local only\n'
            assert recovery['pending_changes']
            assert subprocess.check_output([git, '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == before_head
            assert (host_skills / 'one/SKILL.md').read_text() == host_before
            # Authentication/unsupported remote layouts leave library content unchanged.
            subprocess.run([git, '-C', str(root), 'config', 'agenthub.remoteBranch', 'missing'], check=True)
            run(root, 'git', 'recovery-plan', 'remote', success=False)
            assert (root / 'skills/one/SKILL.md').read_text() == '# One\n'
            subprocess.run([git, '-C', str(root), 'config', 'agenthub.remoteBranch', 'agenthub'], check=True)
            mode['read_only'] = False
            # Restore exact existing history while the server rejects every push.
            expected = subprocess.check_output([git, '-C', str(repo), 'rev-parse', 'refs/heads/agenthub'], text=True).strip()
            mode['read_only'] = True
            restored = temporary / 'restored-library'
            result = run(restored, 'bootstrap', url, '--username', 'tester', secret=token)
            assert result['branch'] == 'agenthub' and result['imported'] == 2
            assert Path(result['recovery_path']).is_dir()
            assert subprocess.check_output([git, '-C', str(restored), 'rev-parse', 'HEAD'], text=True).strip() == expected
            assert subprocess.check_output([git, '-C', str(repo), 'rev-parse', 'refs/heads/agenthub'], text=True).strip() == expected
            assert run(restored, 'git', 'remote-status')['state'] == 'read_verified'
            assert run(restored, 'doctor', '--json')['targets'] == []
            assert all(not profile['enabled'] for profile in run(restored, 'doctor', '--json')['autoSync'])
            assert not subprocess.check_output([git, '-C', str(restored), 'status', '--porcelain'], text=True).strip()
            run(restored, 'bootstrap', url, '--username', 'tester', secret=token, success=False)
            assert (restored / 'skills/one/SKILL.md').read_text() == '# One\n'
            missing = temporary / 'missing-branch'
            run(missing, 'bootstrap', url, '--branch', 'missing', '--username', 'tester', secret=token, success=False)
            assert not run(missing, 'doctor', '--json')['initialized']
            # Malformed layouts, symlinks, schemas and nonportable history fail before activation.
            def git_repo(*args, data=None):
                return subprocess.check_output([git, '-C', str(repo), *args], input=data, text=True).strip()

            clean_tree = git_repo('rev-parse', expected + '^{tree}')
            manifest_blob = git_repo('hash-object', '-w', '--stdin', data='schema_version = 1\n')
            unsafe_blob = git_repo('hash-object', '-w', '--stdin', data='fixture-only')
            wrong_schema_blob = git_repo('hash-object', '-w', '--stdin', data='schema_version = 99\n')
            bad_trees = {
                'not-agenthub': git_repo('mktree', data=f'100644 blob {unsafe_blob}\tREADME.md\n'),
                'bad-schema': git_repo('mktree', data=f'100644 blob {wrong_schema_blob}\tagenthub.toml\n'),
                'symlink': git_repo('mktree', data=f'120000 blob {unsafe_blob}\tagenthub.toml\n'),
                'bad-history': clean_tree,
            }
            historical_tree = git_repo('mktree', data=f'100644 blob {manifest_blob}\tagenthub.toml\n100644 blob {unsafe_blob}\tprivate-runtime.txt\n')
            historical = git_repo('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.com', 'commit-tree', historical_tree, '-m', 'unsafe old content')
            for name, tree in bad_trees.items():
                parent = ['-p', historical] if name == 'bad-history' else []
                commit = git_repo('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.com', 'commit-tree', tree, *parent, '-m', name)
                git_repo('update-ref', 'refs/heads/' + name, commit)
                destination = temporary / ('rejected-' + name)
                run(destination, 'bootstrap', url, '--branch', name, '--username', 'tester', secret=token, success=False)
                assert not run(destination, 'doctor', '--json')['initialized']
                assert not list(temporary.glob(destination.name + '-restore-*'))
                assert not (destination / 'secrets/master.key').exists()
                subprocess.run([git, '-C', str(root), 'config', 'agenthub.remoteBranch', name], check=True)
                run(root, 'git', 'recovery-plan', 'remote', success=False)
                assert (root / 'skills/one/SKILL.md').read_text() == '# One\n'
                subprocess.run([git, '-C', str(root), 'config', 'agenthub.remoteBranch', 'agenthub'], check=True)
            # GitHub-style default main branch works without a fixed agenthub branch.
            default_repo = server_root / 'default.git'
            subprocess.run([git, 'clone', '--bare', str(repo), str(default_repo)], check=True, capture_output=True)
            subprocess.run([git, '-C', str(default_repo), 'update-ref', 'refs/heads/main', expected], check=True)
            subprocess.run([git, '-C', str(default_repo), 'update-ref', '-d', 'refs/heads/agenthub'], check=True)
            subprocess.run([git, '-C', str(default_repo), 'symbolic-ref', 'HEAD', 'refs/heads/main'], check=True)
            main_restore = temporary / 'default-main-restore'
            result = run(main_restore, 'bootstrap', url.replace('/repo.git', '/default.git'), '--username', 'tester', secret=token)
            assert result['branch'] == 'main' and result['imported'] == 2
            assert subprocess.check_output([git, '-C', str(main_restore), 'rev-parse', 'HEAD'], text=True).strip() == expected
            # A freshly restored device receives an advanced remote without an author.
            subprocess.run([git, '-C', str(default_repo), 'config', 'http.receivepack', 'true'], check=True)
            next_version = subprocess.check_output([git, '-C', str(default_repo), '-c', 'user.name=Other device', '-c', 'user.email=other@example.com', 'commit-tree', clean_tree, '-p', expected, '-m', 'new remote version'], text=True).strip()
            subprocess.run([git, '-C', str(default_repo), 'update-ref', 'refs/heads/main', next_version], check=True)
            mode['read_only'] = False
            run(main_restore, 'git', 'sync')
            assert subprocess.check_output([git, '-C', str(main_restore), 'rev-parse', 'HEAD'], text=True).strip() == next_version
            assert not list((main_restore / 'runtime').glob('receive-*'))
            # No synthetic merge or fabricated local author was introduced.
            check = subprocess.run([git, '-C', str(main_restore), 'config', '--local', '--get', 'user.name'], capture_output=True)
            assert check.returncode != 0
            invalid_version = subprocess.check_output([git, '-C', str(default_repo), '-c', 'user.name=Other device', '-c', 'user.email=other@example.com', 'commit-tree', bad_trees['bad-schema'], '-p', next_version, '-m', 'invalid incoming schema'], text=True).strip()
            subprocess.run([git, '-C', str(default_repo), 'update-ref', 'refs/heads/main', invalid_version], check=True)
            run(main_restore, 'git', 'sync', success=False)
            assert subprocess.check_output([git, '-C', str(main_restore), 'rev-parse', 'HEAD'], text=True).strip() == next_version
            assert (main_restore / 'skills/one/SKILL.md').read_text() == '# One\n'
            assert not list((main_restore / 'runtime').glob('receive-*'))
            # Dropped local state is reconstructed; reinstall never requires reimport.
            for path in (restored / 'state').glob('agenthub.db*'):
                path.unlink()
            assert run(restored, 'doctor', '--json')['initialized']
            # A restored device can continue the same history using the saved login.
            mode['read_only'] = False
            run(restored, 'git', 'commit', '-m', 'unchanged', '--name', 'Tester', '--email', 'test@example.com', success=False)
            (restored / 'skills/three').mkdir()
            (restored / 'skills/three/SKILL.md').write_text('# Three restored device update\n')
            result = run(restored, 'git', 'commit', '-m', 'restored device save', '--name', 'Tester', '--email', 'test@example.com')
            # SQLite reconstruction loses credentials, as expected; sign in again.
            assert result['local_saved'] and not result['remote_synced']
            run(restored, 'git', 'login', url, '--branch', 'agenthub', '--username', 'tester', secret=token)
            run(restored, 'git', 'sync')
            # A push permission failure must preserve a newly saved local version.
            mode['read_only'] = True
            (root / 'skills/one/SKILL.md').write_text('# One updated\n')
            result = run(root, 'git', 'commit', '-m', 'local preserved', '--push')
            assert result['local_saved'] and not result['remote_synced'] and result['remote_error']
            assert run(root, 'git', 'remote-status')['state'] == 'auth_failed'
            assert (root / 'skills/two/SKILL.md').exists()
            mode['read_only'] = False
            run(root, 'git', 'sync')
            assert run(root, 'git', 'remote-status')['state'] == 'synced'
            # A failed replacement login restores the previous working credential.
            run(root, 'git', 'login', url, '--branch', 'agenthub', '--username', 'tester', secret='wrong-fixture', success=False)
            run(root, 'git', 'sync')
            for path in [root / '.git/config', root / 'state/agenthub.db']:
                assert token.encode() not in path.read_bytes()
            run(root, 'git', 'forget-credentials')
            settings = run(root, 'git', 'remote-status')
            assert settings['url'] == url and not settings['credential_saved'] and settings['state'] == 'unverified'
            run(root, 'git', 'sync', success=False)
            print('PASS: real HTTPS login, shared native helper, read-only bootstrap with exact history, author-free reception, rejected layouts/branches/auth/history, SQLite recovery, two-device merge, reviewed remote replacement/stale-snapshot refusal, rejected push preserves commit, replacement rollback, encryption, forget, credential-free output')
        finally:
            server.shutdown()
            server.server_close()


if __name__ == '__main__':
    main()

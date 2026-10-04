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
            return json.loads(result.stdout) if result.stdout.lstrip().startswith('{') else result.stdout

        try:
            root = temporary / 'library'
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
            run(second, 'git', 'sync')
            assert (second / 'skills/one/SKILL.md').exists()
            # A push permission failure must preserve a newly saved local version.
            mode['read_only'] = True
            (root / 'skills/one/SKILL.md').write_text('# One updated\n')
            result = run(root, 'git', 'commit', '-m', 'local preserved')
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
            print('PASS: real HTTPS login, shared native helper, two-device merge, rejected push preserves commit, replacement rollback, encryption, forget, credential-free output')
        finally:
            server.shutdown()
            server.server_close()


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Native Ubuntu smoke test; every library and tool write uses a temporary home.
Run inside a private dbus-run-session. Requires Xvfb, tauri-driver, WebKitWebDriver
and system Python dbus/gi. No packages are installed and no real tool is changed.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request


def tray_service():
    import dbus
    import dbus.service
    from dbus.mainloop.glib import DBusGMainLoop
    from gi.repository import GLib
    DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    name = dbus.service.BusName('org.kde.StatusNotifierWatcher', bus)
    class Watcher(dbus.service.Object):
        @dbus.service.method('org.kde.StatusNotifierWatcher', in_signature='s')
        def RegisterStatusNotifierItem(self, item):
            pass
        @dbus.service.method('org.kde.StatusNotifierWatcher', in_signature='s')
        def RegisterStatusNotifierHost(self, host):
            pass
        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='s', out_signature='a{sv}')
        def GetAll(self, interface):
            return {'RegisteredStatusNotifierItems': dbus.Array([], signature='s'), 'IsStatusNotifierHostRegistered': True, 'ProtocolVersion': dbus.Int32(0)}
        @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='ss', out_signature='v')
        def Get(self, interface, prop):
            return self.GetAll(interface)[prop]
    watcher = Watcher(name, '/StatusNotifierWatcher')
    print('ready', flush=True)
    GLib.MainLoop().run()
    return watcher


def wait(predicate, seconds=25):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        try:
            result = predicate()
            if result:
                return result
        except (OSError, ValueError):
            pass
        time.sleep(.25)
    raise AssertionError('native desktop condition timed out')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tools', type=Path, required=True)
    parser.add_argument('--large', action='store_true')
    parser.add_argument('--legacy', action='store_true')
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    binary = repo / 'target/release/agenthub-desktop'
    cli = repo / 'target/release/agenthub'
    tools = args.tools.resolve()
    with tempfile.TemporaryDirectory(prefix='agenthub-native-') as directory:
        temp = Path(directory)
        env = dict(os.environ, AGENTHUB_HOME=str(temp / 'home/.agenthub'), AGENTHUB_USER_HOME=str(temp / 'home'), XDG_DATA_HOME=str(temp / 'data'), XDG_CONFIG_HOME=str(temp / 'config'), XDG_CACHE_HOME=str(temp / 'cache'), DISPLAY=':197', TAURI_AUTOMATION='true', WEBKIT_DISABLE_DMABUF_RENDERER='1', LIBGL_ALWAYS_SOFTWARE='1')
        env['LD_LIBRARY_PATH'] = str(tools / 'root/usr/lib/x86_64-linux-gnu') + ':' + env.get('LD_LIBRARY_PATH', '')
        log = (temp / 'native.log').open('w')
        processes = []
        def spawn(command, **extra):
            p = subprocess.Popen(command, env=env, stdout=log, stderr=log, start_new_session=True, **extra)
            processes.append(p)
            return p
        def command(*parts):
            return subprocess.check_output([str(cli), *parts], env=env, stderr=subprocess.PIPE).decode()
        def request(method, path, body=None):
            data = None if body is None else json.dumps(body).encode()
            req = urllib.request.Request('http://127.0.0.1:4457' + path, data=data, method=method, headers={'Content-Type': 'application/json'})
            with urllib.request.urlopen(req, timeout=35) as response:
                value = json.load(response)['value']
            if isinstance(value, dict) and 'error' in value:
                raise AssertionError(value)
            return value
        session = None
        try:
            spawn([str(tools / 'root/usr/bin/Xvfb'), ':197', '-screen', '0', '1280x900x24', '-nolisten', 'tcp', '-ac'])
            service = spawn(['/usr/bin/python3', str(Path(__file__).resolve()), '--tray-service'])
            def display_ready():
                with socket.socket(socket.AF_UNIX) as sock:
                    sock.connect('\0/tmp/.X11-unix/X197')
                return True
            wait(display_ready)
            wait(lambda: subprocess.check_output(['gdbus', 'call', '--session', '--dest', 'org.freedesktop.DBus', '--object-path', '/org/freedesktop/DBus', '--method', 'org.freedesktop.DBus.NameHasOwner', 'org.kde.StatusNotifierWatcher'], env=env).strip() == b'(true,)')
            command('init', '--empty')
            source = temp / 'home/.agenthub/skills/one/SKILL.md'
            source.parent.mkdir(parents=True)
            source.write_text('# One\n')
            if args.large:
                body = 'Portable test instructions\n' * 256
                for index in range(1000):
                    relative = f'skills/bench-{index}/SKILL.md'
                    for root in ['.agenthub', '.agents', '.cursor', '.codex', '.claude']:
                        path = temp / 'home' / root / relative
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_text(body)
            if args.legacy:
                files = subprocess.check_output(['git', 'ls-tree', '-rz', '--name-only', 'HEAD', 'skills/agenthub-manager'], cwd=repo).split(b'\0')
                for name in filter(None, files):
                    path = temp / 'home/.agenthub' / name.decode()
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(subprocess.check_output(['git', 'show', 'HEAD:' + name.decode()], cwd=repo))
            scope = temp / 'selection.json'
            scope.write_text(json.dumps({'all_kinds': ['skill'], 'sync_on_change': True}))
            plan = json.loads(command('plan', 'agents', '--selection', str(scope), '--json'))
            command('auto-sync', 'enable', 'agents', '--plan-id', plan['id'], '--confirm')
            spawn([str(tools / 'bin/tauri-driver'), '--port', '4457', '--native-port', '4458', '--native-driver', str(tools / 'root/usr/bin/WebKitWebDriver')])
            wait(lambda: request('GET', '/status'))
            session = request('POST', '/session', {'capabilities': {'alwaysMatch': {'tauri:options': {'application': str(binary)}}}})['sessionId']
            endpoint = '/session/' + session
            def ipc(name, payload=None):
                result = request('POST', endpoint + '/execute/async', {'script': "const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(v=>done({ok:true,value:v})).catch(e=>done({ok:false,error:String(e)}));", 'args': [name, payload or {}]})
                assert result['ok'], result
                return result.get('value')
            host = temp / 'home/.agents/skills/one/SKILL.md'
            wait(lambda: ipc('background_status')['tray_available'])
            wait(lambda: (temp / 'home/.agenthub/skills/agenthub-manager/SKILL.md').exists())
            if args.legacy:
                wait(lambda: any((temp / 'home/.agenthub/backups').glob('manager-*')))
            prefs = {'close_behavior': 'ask', 'check_seconds': 120, 'sync_seconds': 60, 'paused': True}
            ipc('set_background_settings', {'settings': prefs})
            source.write_text('# Pending\n')
            wait(lambda: ipc('background_status')['pending'])
            assert host.read_text() == '# One\n'
            ipc('background_refresh')
            wait(lambda: host.read_text() == '# Pending\n')
            prefs.update(sync_seconds=0, paused=False)
            ipc('set_background_settings', {'settings': prefs})
            ipc('close_choice', {'behavior': 'tray', 'remember': True})
            source.write_text('# Edited while hidden\n')
            wait(lambda: host.read_text() == '# Edited while hidden\n')
            second = spawn([str(binary)])
            assert second.wait(timeout=15) == 0
            assert ipc('background_settings')['close_behavior'] == 'tray'
            records = json.loads(command('history', '--json'))
            assert any(tx.get('trigger') == 'library_edit' and any(c['id'] == 'one' for c in tx.get('resource_changes', [])) for tx in records)
            image = request('GET', endpoint + '/screenshot')
            (repo / 'target/native-background.png').write_bytes(base64.b64decode(image))
            # Removing a host copy does not count as a library change.
            host.unlink()
            time.sleep(6)
            assert not host.exists()
            if args.large:
                candidates = []
                for proc in Path('/proc').iterdir():
                    if not proc.name.isdigit():
                        continue
                    try:
                        if (proc / 'exe').resolve() == binary.resolve() and f'AGENTHUB_HOME={temp}/home/.agenthub'.encode() in (proc / 'environ').read_bytes():
                            candidates.append(proc)
                    except (FileNotFoundError, PermissionError, ProcessLookupError):
                        pass
                assert len(candidates) == 1, candidates
                def cpu():
                    fields = (candidates[0] / 'stat').read_text().split(') ', 1)[1].split()
                    return (int(fields[11]) + int(fields[12])) / os.sysconf('SC_CLK_TCK')
                before = cpu()
                started = time.monotonic()
                time.sleep(5)
                elapsed = time.monotonic() - started
                latency = time.monotonic()
                ipc('background_status')
                latency = (time.monotonic() - latency) * 1000
                rss = next(line for line in (candidates[0] / 'status').read_text().splitlines() if line.startswith('VmRSS:'))
                print(f'PERF: 1000 library Skills + 4 x 1000 tool copies; idle CPU={(cpu()-before)/elapsed*100:.2f}% of one core, status IPC={latency:.1f}ms, {rss}')
            # Unsupported tray is explicitly refused without hiding the window.
            service.terminate()
            service.wait(timeout=5)
            try:
                ipc('close_choice', {'behavior': 'tray', 'remember': False})
                raise RuntimeError('lost tray should refuse hiding')
            except AssertionError as error:
                assert 'system tray is unavailable' in str(error), error
            ipc('close_choice', {'behavior': 'exit', 'remember': False})
            print('PASS: native IPC, manager maintenance, queued/pause/manual delivery, hidden-window external sync, single-instance reopen, attributed history, no host-only refill and orderly exit')
        except Exception:
            log.flush()
            print((temp / 'native.log').read_text()[-8000:], file=sys.stderr)
            raise
        finally:
            if session:
                try:
                    request('DELETE', '/session/' + session)
                except Exception:
                    pass
            for p in reversed(processes):
                if p.poll() is None:
                    os.killpg(p.pid, signal.SIGTERM)
                    try:
                        p.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(p.pid, signal.SIGKILL)
            log.close()

if __name__ == '__main__':
    if '--tray-service' in sys.argv:
        tray_service()
    else:
        main()

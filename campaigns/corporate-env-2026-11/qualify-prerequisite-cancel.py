#!/usr/bin/env python3
"""Observe real missing-command Cancel versus manual-defer terminal choices."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import termios
import time


def child(choice):
    root = Path('/world')
    env = {'HOME': '/home/probe', 'PATH': '/usr/bin:/bin', 'TERM': 'xterm',
           'VIBE_SETTINGS': '/world/settings with spaces', 'VIBE_REGISTRY_CACHE': '/world/cache',
           'VIBEVM_SEARCH_CACHE_DIR': '/world/search-cache', 'VIBE_NO_DEFAULT_REGISTRY': '1'}
    master, slave = pty.openpty()
    before = termios.tcgetattr(slave)

    def session():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    proc = subprocess.Popen(['/tools/vibe', 'install', '-g', 'org.example/prerequisite',
                             '--registry', '/world/registry', '--offline', '--platform', 'linux', '--arch', 'x86_64', '--installer', 'main'],
                            stdin=slave, stdout=slave, stderr=slave, env=env,
                            cwd='/tmp', preexec_fn=session)
    data = b''
    stage = 0
    sent_at = 0.0
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if select.select([master], [], [], 0.03)[0]:
            try:
                data += os.read(master, 65536)
            except OSError:
                break
        now = time.monotonic()
        if stage == 0 and b'is missing' in data and b'Cancel installation' in data and not termios.tcgetattr(slave)[3] & termios.ECHO:
            os.write(master, b'\r' if choice == 'cancel' else b'\x1b[B')
            stage = 1
            sent_at = now
        elif choice == 'defer' and stage == 1 and now - sent_at > 0.2:
            os.write(master, b'\r')
            stage = 2
        if proc.poll() is not None:
            break
    if proc.poll() is None:
        proc.kill()  # Only this bounded test-owned child, never an ambient build.
        proc.wait()
        (root / 'terminal.txt').write_bytes(data)
        raise AssertionError('test-owned PTY journey timed out')
    while select.select([master], [], [], 0.02)[0]:
        try:
            data += os.read(master, 65536)
        except OSError:
            break
    restored = termios.tcgetattr(slave) == before
    os.close(master)
    os.close(slave)
    (root / 'terminal.txt').write_bytes(data)
    assert stage == (1 if choice == 'cancel' else 2), 'actual prompt not answered'
    assert restored, 'terminal attributes changed'
    assert proc.returncode != 0, 'prerequisite choice must not claim installation success'
    if choice == 'defer':
        assert b'prerequisite_deferred' in data or b'deferred' in data.lower(), data.decode(errors='replace')
    else:
        assert b'prerequisite_deferred' not in data, 'cancel relabeled as defer'
    settings = root / 'settings with spaces'
    for name in ['applications/index.json', 'cache/app-work', 'opt/bin', 'opt/apps']:
        assert not (settings / name).exists(), name
    assert not (root / 'ACQUIRED').exists(), 'copy effect ran'
    row = {'choice': choice, 'exit': proc.returncode, 'actual_prompt_seen': True,
           'full_termios_restored': restored, 'target_effects_absent': True,
           'scope': 'synthetic missing-command prerequisite, not real npm compatibility'}
    (root / 'result.json').write_text(json.dumps(row, indent=2) + '\n')
    print(json.dumps(row))


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--inside', choices=['cancel', 'defer'])
    p.add_argument('--cli')
    p.add_argument('--proxy')
    p.add_argument('--output')
    a = p.parse_args()
    if a.inside:
        return child(a.inside)
    cli, proxy = Path(a.cli).absolute(), Path(a.proxy).absolute()
    assert hashlib.sha256(cli.read_bytes()).hexdigest() == '2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e'
    output = Path(a.output)
    output.mkdir(mode=0o700)
    for choice in ['cancel', 'defer']:
        world = output / choice
        package = world / 'registry/org.example/prerequisite/v1.0.0'
        package.mkdir(parents=True)
        (world / 'settings with spaces').mkdir()
        (package / 'vibe.toml').write_text('''[package]
group='org.example'
name='prerequisite'
kind='app'
version='1.0.0'
[application]
id='prerequisite'
[[installer_options]]
id='main'
name='Prerequisite control'
platform='linux'
platform_name='Linux'
arch='x86_64'
[[installer_options.steps]]
id='copy'
title='Copy after prerequisite'
kind='copy'
from='payload'
needs=['command:vibe-fixture-absent-npm']
''')
        (package / 'payload').mkdir()
        (package / 'payload/ACQUIRED').write_text('synthetic must-not-install')
        command = ['/usr/bin/bwrap', '--unshare-all', '--die-with-parent', '--new-session', '--clearenv']
        for source in ['/usr', '/bin', '/lib', '/lib64']:
            command += ['--ro-bind', source, source]
        command += ['--dir', '/etc', '--ro-bind', '/etc/ld.so.cache', '/etc/ld.so.cache',
                    '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--dir', '/home/probe',
                    '--bind', str(world), '/world', '--ro-bind', str(cli), '/tools/vibe',
                    '--ro-bind', str(proxy), '/tools/vibe-app-proxy',
                    '--ro-bind', str(Path(__file__).absolute()), '/control.py',
                    '--chdir', '/tmp', '/usr/bin/python3', '-I', '/control.py', '--inside', choice]
        result = subprocess.run(command, capture_output=True, text=True, timeout=25)
        (world / 'outer.json').write_text(json.dumps({'exit': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}, indent=2) + '\n')
        assert result.returncode == 0, result.stderr
    print(json.dumps({'journeys': [json.loads((output / c / 'result.json').read_text()) for c in ['cancel', 'defer']],
                      'cli_sha256': hashlib.sha256(cli.read_bytes()).hexdigest(),
                      'proxy_sha256': hashlib.sha256(proxy.read_bytes()).hexdigest()}, indent=2))


if __name__ == '__main__':
    main()

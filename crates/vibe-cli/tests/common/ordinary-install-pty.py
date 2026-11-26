"""Finite synthetic ordinary install PTY: collection and apply are separate."""
import errno
import json
import os
import pathlib
import pty
import re
import select
import signal
import subprocess
import sys
import termios
import time
cli, project, mode, source, expected_count, expected_destination = sys.argv[1:]
assert expected_count == '2'
assert pathlib.Path(expected_destination).is_relative_to(pathlib.Path(project))
question = f'? Install {expected_count} packages into {expected_destination}? ›'
pattern = re.compile(r'(?m)^' + re.escape(question) + r'\r?\n❯ yes\r?\n  no(?:\r?\n|$)')
control = question + '\r\n❯ yes\r\n  no\r\n'
assert pattern.search(control) is not None
assert pattern.search(control.replace('Install 2 packages', 'Install 3 packages')) is None
assert pattern.search(control.replace(expected_destination, expected_destination + '-foreign')) is None
master, slave = pty.openpty()
original = termios.tcgetattr(slave)
def child_terminal():
    os.setsid()
    import fcntl
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)
p = subprocess.Popen([cli, 'install', '--offline', '--path', project, '--ordinary-interactive'], stdin=slave, stdout=slave, stderr=slave, preexec_fn=child_terminal, close_fds=True)
seen = bytearray()
stage = 0
started = time.monotonic()
while p.poll() is None:
    if time.monotonic() - started > 20:
        os.killpg(p.pid, signal.SIGKILL)
        p.wait()
        raise RuntimeError('ordinary install PTY timeout stage=' + str(stage) + ' transcript=' + seen.decode('utf-8', 'replace'))
    if not select.select([master], [], [], .1)[0]:
        continue
    try:
        seen.extend(os.read(master, 16384))
    except OSError as error:
        if error.errno == errno.EIO:
            break
        raise
    if stage == 0 and b'Extra support (extra)' in seen:
        os.write(master, b'\x1b[B\r')
        stage = 1
    if stage == 1 and b'Verbose (verbose)' in seen:
        os.write(master, b'\r' if mode == 'true' else b'\x1b[B\r')
        stage = 2
    # Match the actual question and its separate active row, ignoring ANSI only.
    plain = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', bytes(seen)).decode('utf-8', 'replace')
    apply_question = pattern.search(plain)
    if stage == 2 and apply_question is not None:
        if mode == 'source-drift':
            path = pathlib.Path(source)
            path.write_bytes(path.read_bytes() + b'\nSYN_SOURCE_DRIFT_AFTER_PLAN\n')
        if mode == 'root-drift':
            path = pathlib.Path(project) / 'vibe.toml'
            path.write_bytes(path.read_bytes() + b'\n# SYN_ROOT_DRIFT_AFTER_PLAN\n')
        os.write(master, b'\x1b' if mode == 'cancel' else b'\r')
        stage = 3
p.wait(timeout=3)
while select.select([master], [], [], 0)[0]:
    try:
        chunk = os.read(master, 16384)
        if not chunk:
            break
        seen.extend(chunk)
    except OSError:
        break
restored = termios.tcgetattr(slave)
os.close(master)
os.close(slave)
print(json.dumps({'status': p.returncode, 'stage': stage, 'terminal_restored': restored == original, 'cursor_visible': seen.rfind(b'\x1b[?25h') >= seen.rfind(b'\x1b[?25l'), 'transcript': seen.decode('utf-8', 'replace')}))

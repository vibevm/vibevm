"""Finite synthetic ordinary install PTY: collection and apply are separate."""
import errno
import json
import os
import pathlib
import pty
import select
import signal
import subprocess
import sys
import termios
import time
cli, project, mode, source = sys.argv[1:]
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
    # The apply question is a two-row list, `yes` first and marked (the default).
    if stage == 2 and b'packages into ' in seen and b'\xe2\x80\xba yes' in seen and b'  no' in seen:
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

"""Synthetic finite PTY driver for ordinary preview; never supplies approval."""
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
process = subprocess.Popen([cli, 'show', 'configuration-preview', 'org.vibevm/oracle-driver', '--path', project, '--interactive'], stdin=slave, stdout=slave, stderr=slave, preexec_fn=child_terminal, close_fds=True)
seen = bytearray()
stage = 0
started = time.monotonic()
while process.poll() is None:
    if time.monotonic() - started > 12:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        raise RuntimeError('interactive preview timed out')
    readable, _, _ = select.select([master], [], [], 0.1)
    if not readable:
        continue
    try:
        data = os.read(master, 16384)
    except OSError as error:
        if error.errno == errno.EIO:
            break
        raise
    seen.extend(data)
    if stage == 0 and b'Extra support (extra)' in seen:
        os.write(master, b'\x1b[B\r')
        stage = 1
    if stage == 1 and b'Verbose (verbose)' in seen:
        if mode == 'drift':
            path = pathlib.Path(source)
            path.write_bytes(path.read_bytes() + b'\nsynthetic source changed\n')
        response = {'false': b'\x1b[B\r', 'true': b'\r', 'cancel': b'\x1b', 'interrupt': b'\x03', 'drift': b'\x1b[B\r'}[mode]
        os.write(master, response)
        stage = 2
process.wait(timeout=3)
while select.select([master], [], [], 0)[0]:
    try:
        data = os.read(master, 16384)
        if not data:
            break
        seen.extend(data)
    except OSError:
        break
restored = termios.tcgetattr(slave)
os.close(master)
os.close(slave)
print(json.dumps({'status': process.returncode, 'stage': stage, 'terminal_restored': restored == original, 'cursor_visible': seen.rfind(b'\x1b[?25h') >= seen.rfind(b'\x1b[?25l'), 'transcript': seen.decode('utf-8', 'replace')}))

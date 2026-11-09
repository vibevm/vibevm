#!/usr/bin/env python3
# A stand-in for px in tests: honours --listen/--port, runs a forwarding
# proxy (CONNECT and absolute-form GET) that maps every host to 127.0.0.1,
# and records `start <pid>` / `exit <pid>` / request lines in the file
# `--fake-log=<path>` (or $FAKE_PX_LOG) names. `--fake-fail=1` exits at once;
# `--fake-test-fail=1` makes `--test` fail.
import os, select, signal, socket, sys, threading
args = {}
for a in sys.argv[1:]:
    k, _, v = a.partition('=')
    args[k] = v
log = args.get('--fake-log') or os.environ.get('FAKE_PX_LOG')
def note(line):
    if log:
        with open(log, 'a') as f:
            f.write(line + '\n')
if '--test' in args:
    note('test ' + args['--test'])
    sys.exit(1 if (args.get('--fake-test-fail') or os.environ.get('FAKE_PX_TEST_FAIL')) == '1' else 0)
if (args.get('--fake-fail') or os.environ.get('FAKE_PX_FAIL')) == '1':
    sys.stderr.write('fake px: refusing to start (FAKE_PX_FAIL)\n')
    sys.exit(3)
def bye(*_):
    note('exit %d' % os.getpid())
    os._exit(0)
signal.signal(signal.SIGTERM, bye)
server = socket.socket()
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind((args.get('--listen', '127.0.0.1'), int(args.get('--port', '3128'))))
server.listen(64)
note('start %d %s' % (os.getpid(), ' '.join(sys.argv[1:])))
def pump(a, b):
    try:
        while True:
            r, _, _ = select.select([a, b], [], [], 30)
            if not r:
                return
            for s in r:
                data = s.recv(65536)
                if not data:
                    return
                (b if s is a else a).sendall(data)
    except OSError:
        return
    finally:
        a.close(); b.close()
def serve(conn):
    head = b''
    while b'\r\n\r\n' not in head:
        chunk = conn.recv(4096)
        if not chunk:
            conn.close(); return
        head += chunk
    line = head.split(b'\r\n', 1)[0].decode()
    note('request ' + line)
    method, target, _ = line.split(' ', 2)
    if method == 'CONNECT':
        host, port = target.rsplit(':', 1)
        up = socket.create_connection(('127.0.0.1', int(port)))
        conn.sendall(b'HTTP/1.1 200 Connection established\r\n\r\n')
        rest = head.split(b'\r\n\r\n', 1)[1]
        if rest:
            up.sendall(rest)
        pump(conn, up)
    else:
        from urllib.parse import urlsplit
        u = urlsplit(target)
        up = socket.create_connection(('127.0.0.1', u.port or 80))
        path = u.path or '/'
        if u.query:
            path += '?' + u.query
        up.sendall(head.replace(target.encode(), path.encode(), 1))
        pump(conn, up)
while True:
    c, _ = server.accept()
    threading.Thread(target=serve, args=(c,), daemon=True).start()

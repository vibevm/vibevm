#!/usr/bin/env python3
# Test HTTPS server: serves files under --root, and a git smart-HTTP mount
# at /git/ over the bare repositories under --root (git http-backend), on a
# free loopback port written to --port-file. TLS uses --cert/--key.
import http.server, os, ssl, subprocess, sys, urllib.parse
args = dict(a.partition('=')[::2] for a in sys.argv[1:])
root = args['--root']
class Handler(http.server.SimpleHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def __init__(self, *a, **k):
        super().__init__(*a, directory=root, **k)
    def log_message(self, *a):
        pass
    def git(self):
        parsed = urllib.parse.urlsplit(self.path)
        body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        env = {'PATH': '/usr/bin:/bin', 'GIT_PROJECT_ROOT': root, 'GIT_HTTP_EXPORT_ALL': '1',
               'PATH_INFO': parsed.path[len('/git'):], 'QUERY_STRING': parsed.query,
               'REQUEST_METHOD': self.command, 'CONTENT_TYPE': self.headers.get('Content-Type', ''),
               'CONTENT_LENGTH': str(len(body)), 'GIT_CONFIG_NOSYSTEM': '1', 'HOME': root}
        p = subprocess.run(['git', 'http-backend'], env=env, input=body, capture_output=True, timeout=60)
        head, _, data = p.stdout.partition(b'\r\n\r\n')
        fields = [l.decode().split(':', 1) for l in head.split(b'\r\n') if b':' in l]
        status = next((int(v.strip().split()[0]) for k, v in fields if k.lower() == 'status'), 200)
        self.send_response(status)
        for k, v in fields:
            if k.lower() not in ('status', 'content-length'):
                self.send_header(k, v.strip())
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        if self.path.startswith('/git/'):
            return self.git()
        return super().do_GET()
    def do_POST(self):
        if self.path.startswith('/git/'):
            return self.git()
        self.send_error(405)
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
server.daemon_threads = True
if '--cert' in args:
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(args['--cert'], args['--key'])
    server.socket = tls.wrap_socket(server.socket, server_side=True)
with open(args['--port-file'] + '.tmp', 'w') as f:
    f.write(str(server.server_address[1]))
os.rename(args['--port-file'] + '.tmp', args['--port-file'])
server.serve_forever()

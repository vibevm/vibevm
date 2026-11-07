# Numeric loopback only. Runtime scratch is isolated; all Git/TLS setup must pass.
import urllib.parse,random
import base64,functools,http.server,json,os,pathlib,ssl,subprocess,sys,tempfile,threading
root=pathlib.Path(tempfile.mkdtemp(prefix='vibe-native-test-')); home=root/'home';home.mkdir()
env={'PATH':'/usr/bin:/bin','HOME':str(home),'XDG_CONFIG_HOME':str(home),'GIT_CONFIG_SYSTEM':'/dev/null','GIT_CONFIG_GLOBAL':'/dev/null','GIT_CONFIG_NOSYSTEM':'1','LC_ALL':'C','GIT_TERMINAL_PROMPT':'0'}
def git(args,cwd=root):
 p=subprocess.run(['/usr/bin/git',*args],cwd=cwd,env=env,capture_output=True,timeout=20)
 if p.returncode: raise RuntimeError(('fixture Git setup failed',args,p.returncode,p.stderr.decode()))
 return p.stdout.decode().strip()
work=root/'work';git(['init',str(work)])
(work/'payload.txt').write_text('retained fixture\n')
# Exceed Git's small-pack threshold so real receiving-byte progress is emitted.
for i in range(130):(work/('transfer-'+str(i))).write_bytes(random.Random(i).randbytes(2048))
(work/'vibe.toml').write_text("[package]\ngroup='org.fixture'\nname='demo'\nkind='feat'\nversion='1.0.0'\n")
git(['add','.'],work);git(['-c','user.name=Fixture','-c','user.email=f@example.invalid','commit','-m','fixture'],work);git(['tag','v1.0.0'],work)
head=git(['rev-parse','HEAD'],work)
web=root/'web';web.mkdir()
cert=root/'ca.pem';key=root/'key.pem'
p=subprocess.run(['/usr/bin/openssl','req','-x509','-newkey','rsa:2048','-nodes','-keyout',str(key),'-out',str(cert),'-days','1','-subj','/CN=numeric-fixture','-addext','subjectAltName=IP:127.0.0.1,IP:127.0.0.2'],env=env,capture_output=True,timeout=20)
if p.returncode:raise RuntimeError('TLS fixture creation failed')
key.chmod(0o600);tls=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);tls.load_cert_chain(cert,key)
auth='Basic '+base64.b64encode(b'x-access-token:SYNTHETIC_N1_AUTH').decode()
class Server(http.server.ThreadingHTTPServer):daemon_threads=True
class Handler(http.server.SimpleHTTPRequestHandler):
 protocol_version='HTTP/1.1'
 def log_message(self,*args):pass
 def handle(self):
  try:super().handle()
  except ConnectionResetError:pass
 def smart(self):
  parsed=urllib.parse.urlsplit(self.path)
  body=self.rfile.read(int(self.headers.get('Content-Length','0')))
  child=env.copy();child.update({'GIT_PROJECT_ROOT':str(web),'GIT_HTTP_EXPORT_ALL':'1','PATH_INFO':parsed.path.removeprefix('/smart'),'QUERY_STRING':parsed.query,'REQUEST_METHOD':self.command,'CONTENT_TYPE':self.headers.get('Content-Type',''),'CONTENT_LENGTH':str(len(body))})
  p=subprocess.run(['/usr/bin/git','http-backend'],env=child,input=body,capture_output=True,timeout=20)
  if p.returncode:raise RuntimeError(('fixture http-backend failed',p.stderr.decode()))
  headers,data=p.stdout.split(b'\r\n\r\n',1);fields=[line.decode().split(':',1) for line in headers.split(b'\r\n')]
  status=next((int(v.strip().split()[0]) for k,v in fields if k.lower()=='status'),200)
  self.send_response(status)
  for k,v in fields:
   if k.lower()!='status':self.send_header(k,v.strip())
  self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
 def do_POST(self):
  with (root/(self.server.label+'.log')).open('a') as f:f.write('POST '+self.path+'\n')
  if self.path.startswith('/smart/'):return self.smart()
  self.send_error(403)
 def do_GET(self):
  with (root/(self.server.label+'.log')).open('a') as f:f.write(self.path+'\n')
  if self.path.startswith('/smart/'):return self.smart()
  if self.path.startswith('/public/org.fixture.demo.git'):self.path=self.path.replace('/public/org.fixture.demo.git','/repo.git',1)
  if self.server.label=='a' and self.path.startswith('/alternate.git/objects/'):
   if self.path=='/alternate.git/objects/info/http-alternates':
    data=(bu+'/repo.git/objects\n').encode();self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data);return
   self.send_error(404);return
  if self.path.startswith('/alternate.git'):self.path=self.path.replace('/alternate.git','/repo.git',1)
  if self.path.startswith('/redirect.git'):
   self.send_response(302);self.send_header('Location',bu+self.path.replace('/redirect.git','/repo.git'));self.send_header('Content-Length','0');self.end_headers();return
  if self.path.startswith('/auth.git') or self.path.startswith('/auth/'):
   if self.headers.get('Authorization')!=auth:
    self.send_response(401);self.send_header('WWW-Authenticate','Basic realm="fixture"');self.send_header('Content-Length','0');self.end_headers();return
   self.path=self.path.replace('/auth.git','/repo.git',1).replace('/auth/org.fixture.demo.git','/repo.git',1)
  super().do_GET()
servers=[]
for label,host in [('a','127.0.0.1'),('b','127.0.0.2')]:
 s=Server((host,0),functools.partial(Handler,directory=str(web)));s.label=label;s.socket=tls.wrap_socket(s.socket,server_side=True);threading.Thread(target=s.serve_forever,daemon=True).start();servers.append(s)
au='https://127.0.0.1:'+str(servers[0].server_port);bu='https://127.0.0.2:'+str(servers[1].server_port)
(work/'.gitmodules').write_text('[submodule "child"]\n path=child\n url='+bu+'/repo.git\n')
git(['add','.gitmodules'],work);git(['update-index','--add','--cacheinfo','160000,'+head+',child'],work);git(['-c','user.name=Fixture','-c','user.email=f@example.invalid','commit','-m','submodule fixture'],work);git(['tag','v2.0.0'],work)
git(['reset','--hard',head],work)
(work/'payload.txt').write_text('later fixture\n');git(['add','payload.txt'],work);git(['-c','user.name=Fixture','-c','user.email=f@example.invalid','commit','-m','later HEAD'],work);current_head=git(['rev-parse','HEAD'],work)
git(['clone','--bare',str(work),str(web/'repo.git')]);git(['update-server-info'],web/'repo.git')
print(json.dumps({'root':str(root),'a':au,'b':bu,'ca':str(cert),'head':head,'current_head':current_head,'work':str(work)}),flush=True)
try:sys.stdin.read()
finally:
 for s in servers:s.shutdown();s.server_close()

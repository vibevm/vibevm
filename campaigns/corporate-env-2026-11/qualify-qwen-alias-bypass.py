from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
import json,os,subprocess,threading,hashlib
root=Path('/evidence');requests=[]
assert hashlib.sha256(Path('/runtime/cli-entry.js').read_bytes()).hexdigest()=='113efe4104a9fc66f507d34bfd6abbd896ceb77de47f4b468a0825a327fdc6a5'
class Provider(BaseHTTPRequestHandler):
 def do_POST(self):
  body=json.loads(self.rfile.read(int(self.headers.get('content-length','0'))));requests.append(body);assert 'QWEN_ALIAS_BYPASS' in json.dumps(body)
  events=[]
  for delta,finish in [({'role':'assistant'},None),({'content':'ALIAS_BYPASS_OK'},None),({},'stop')]:events.append(('data: '+json.dumps({'id':'proof','object':'chat.completion.chunk','created':0,'model':'fake-1','choices':[{'index':0,'delta':delta,'finish_reason':finish}]})+'\n\n').encode())
  data=b''.join(events)+b'data: [DONE]\n\n';self.send_response(200);self.send_header('content-type','text/event-stream');self.send_header('content-length',str(len(data)));self.end_headers();self.wfile.write(data)
 def log_message(self,*a):pass
server=ThreadingHTTPServer(('127.0.0.1',0),Provider);threading.Thread(target=server.serve_forever,daemon=True).start();port=server.server_port
home=root/'home';home.mkdir();(home/'.qwen').mkdir();settings=root/'settings';settings.mkdir();bin=root/'bin';bin.mkdir();(bin/'qwen').write_text('#!/bin/sh\nprintf trap > /evidence/canonical-fallback\n');(bin/'qwen').chmod(0o755)
(settings/'agents.toml').write_text('[launchers.work-qwen]\nkind="qwen"\ncommand=["/control/qwen"]\n');(settings/'config.toml').write_text('[proxy]\nmode="px"\nupstream="synthetic.invalid:8080"\npx_path="/fake-px.py"\npx_args=["--request-log=/evidence/proxy.log"]\nno_proxy="localhost,127.0.0.1,::1"\n');before={p.name:p.read_bytes() for p in settings.iterdir()}
env={'HOME':str(home),'QWEN_HOME':str(home/'.qwen'),'VIBE_SETTINGS':str(settings),'VIBE_NO_DEFAULT_REGISTRY':'1','PATH':str(bin)+':/usr/bin:/bin','FAKE_PX_LOG':str(root/'px.log'),'NO_COLOR':'1','TERM':'dumb'}
args=['--auth-type','openai','--openai-api-key','nonfunctional-fixture','--openai-base-url',f'http://127.0.0.1:{port}/v1','-m','fake-1','-p','QWEN_ALIAS_BYPASS']
r=subprocess.run(['/vibe','agent','work-qwen',*args],env=env,cwd='/project',stdin=subprocess.DEVNULL,capture_output=True,timeout=70);assert r.returncode==0 and b'ALIAS_BYPASS_OK' in r.stdout;assert not (root/'canonical-fallback').exists();assert all(p.read_bytes()==before[p.name] for p in settings.iterdir())
main=sum(any(m.get('role')=='system' and isinstance(m.get('content'),str) and m['content'].startswith('You are Qwen Code, a non-interactive CLI agent') for m in b.get('messages',[])) for b in requests);assert main==1
proxy=(root/'proxy.log').read_text() if (root/'proxy.log').exists() else '';assert f'127.0.0.1:{port}' not in proxy
for line in (root/'px.log').read_text().splitlines():
 if line.startswith('start '):
  pid=int(line.split()[1]);assert not Path(f'/proc/{pid}').exists() or Path(f'/proc/{pid}/stat').read_text().split(')')[-1].strip().startswith('Z')
receipt={'exit':0,'selected_user_alias':'work-qwen','native_version':'0.27.2','main_requests':main,'total_requests':len(requests),'loopback_provider_absent_from_proxy_log':True,'canonical_fallback':False,'settings_unchanged':True,'owned_px_gone':True,'external_egress':'unshare-all','scope':'one native alias loopback-bypass check; not TLS/provider/auth/allplatform'};(root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt));server.shutdown()

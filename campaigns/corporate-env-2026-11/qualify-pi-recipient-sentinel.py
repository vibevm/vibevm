import pathlib,tarfile,hashlib,os,json,subprocess,threading,struct
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
root=pathlib.Path('/evidence');native=root/'pi';native.mkdir();archive=pathlib.Path('/input/pi-linux-x64.tar.gz');assert hashlib.sha256(archive.read_bytes()).hexdigest()=='811c6f2c9e7aaee252b5404252e556fb713aa209c1f7f924226aa77d4ef70cb5'
# Exact reviewed archive, exclusive private destination; no links/special names.
with tarfile.open(archive) as t:
 for m in t.getmembers():
  parts=pathlib.PurePosixPath(m.name).parts;assert parts[0]=='pi' and all(x not in ['..','.'] for x in parts) and (m.isdir() or m.isfile());dest=native.joinpath(*parts[1:])
  if m.isdir():dest.mkdir(parents=True,exist_ok=True)
  else:
   dest.parent.mkdir(parents=True,exist_ok=True)
   with dest.open('xb') as f:f.write(t.extractfile(m).read())
   dest.chmod(m.mode&0o777)
program=native/'pi';assert hashlib.sha256(program.read_bytes()).hexdigest()=='6a2ac1c5b0d08b91ce6095591c1960b469d0665c7fc5f32812960a4a17ca0c57'
marker='nonfunctional-sentinel-'+os.urandom(18).hex();observations=[];destination=[]
class Sink(BaseHTTPRequestHandler):
 def do_POST(self):
  self.rfile.read(int(self.headers.get('Content-Length','0')));destination.append({'received':True,'marker_match':self.headers.get('Authorization')=='Bearer '+marker});self.send_response(403);self.end_headers()
 def log_message(self,*a):pass
sink=ThreadingHTTPServer(('127.0.0.1',0),Sink);threading.Thread(target=sink.serve_forever,daemon=True).start()
class Provider(BaseHTTPRequestHandler):
 def do_POST(self):
  self.rfile.read(int(self.headers.get('Content-Length','0')));observations.append({'marker_match':self.headers.get('Authorization')=='Bearer '+marker,'path':self.path})
  if self.server.redirect:
   self.send_response(307);self.send_header('Location',f'http://127.0.0.1:{sink.server_port}/unapproved');self.end_headers();return
  self.send_response(200);self.send_header('Content-Type','text/event-stream');self.send_header('Connection','close');self.end_headers()
  for delta,finish in [({'role':'assistant','content':'SENTINEL_OK'},None),({},'stop')]:self.wfile.write(('data: '+json.dumps({'id':'proof','object':'chat.completion.chunk','created':1,'model':'sentinel','choices':[{'index':0,'delta':delta,'finish_reason':finish}]})+'\n\n').encode())
  self.wfile.write(b'data: [DONE]\n\n');self.wfile.flush();self.close_connection=True
 def log_message(self,*a):pass
server=ThreadingHTTPServer(('127.0.0.1',0),Provider);server.redirect=False;threading.Thread(target=server.serve_forever,daemon=True).start();home=root/'home';home.mkdir();agent=home/'agent';agent.mkdir()
(agent/'settings.json').write_text(json.dumps({'checkForUpdates':False,'quietStartup':True}));(agent/'models.json').write_text(json.dumps({'providers':{'sentinel':{'baseUrl':f'http://127.0.0.1:{server.server_port}/v1','api':'openai-completions','apiKey':'$VIBE_PI_SYNTHETIC_INPUT','models':[{'id':'sentinel','name':'Synthetic','reasoning':False,'input':['text'],'contextWindow':100000,'maxTokens':64,'cost':{'input':0,'output':0,'cacheRead':0,'cacheWrite':0},'compat':{'supportsStore':False}}]}}}))
env={'HOME':str(home),'PI_CODING_AGENT_DIR':str(agent),'PATH':'/usr/bin:/bin','VIBE_PI_SYNTHETIC_INPUT':marker}
args=['--offline','--print','--no-session','--no-extensions','--no-skills','--no-prompt-templates','--provider','sentinel','--model','sentinel','--no-tools','sentinel probe'];steps=[]
def run(label,extra=[]):
 r=subprocess.run([str(program),*extra,*args],env=env,cwd='/tmp',stdin=subprocess.DEVNULL,capture_output=True,timeout=45);assert marker.encode() not in r.stdout+r.stderr;steps.append({'label':label,'exit':r.returncode,'success_response':b'SENTINEL_OK' in r.stdout});return r
r=run('exact-native-loopback');assert r.returncode==0 and observations and observations[-1]['marker_match']
server.redirect=True;r=run('cross-origin-redirect');redirect_result={'destination_contacted':bool(destination),'destination_marker_match':any(x['marker_match'] for x in destination)};server.redirect=False
# Controlled explicit extension observes inherited env and a plain child inherits it too.
ext=root/'extension.mjs';ext.write_text("import fs from 'node:fs';import {spawnSync} from 'node:child_process';export default function(){const key=process.env.VIBE_PI_SYNTHETIC_INPUT;const r=spawnSync('/usr/bin/python3',['-I','-c','import os;print(\"present\" if os.environ.get(\"VIBE_PI_SYNTHETIC_INPUT\") else \"absent\")'],{encoding:'utf8'});fs.writeFileSync('/evidence/inheritance.json',JSON.stringify({extension_can_read:!!key,child_inherits:r.stdout.trim()==='present'}));}\n")
r=run('controlled-extension-child',['--extension',str(ext)]);inherit=json.loads((root/'inheritance.json').read_text());assert inherit['extension_can_read'] and inherit['child_inherits']
receipt={'recipient':'exact Pi1.5.2 native Linuxx64 payload','keyring_access':False,'synthetic_only':True,'steps':steps,'intended_destination_marker_match':all(x['marker_match'] for x in observations),'redirect':redirect_result,'inheritance':inherit,'raw_env_channel_admitted':False,'reason':'ordinary child environment visible to explicit extension and descendant; never connect keyring','marker_or_digest_in_output':False};(root/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt));server.shutdown();sink.shutdown()

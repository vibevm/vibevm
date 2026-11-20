import pathlib,os,json,subprocess,hashlib,zipfile,io,pty,termios,select,time,fcntl
world=pathlib.Path('/evidence');kit=world/'kit';kit.mkdir();settings=world/'settings';settings.mkdir(mode=0o700);install=world/'install';home=world/'home';home.mkdir()
env={'HOME':str(home),'VIBE_SETTINGS':str(settings),'VIBEVM_INSTALL_ROOT':str(install),'PATH':'/usr/bin:/bin','TERM':'xterm','VIBE_NO_DEFAULT_REGISTRY':'1'}
marker=os.urandom(32).hex().encode();marker_digest=hashlib.sha256(marker).hexdigest().encode();steps=[]
def clean(data):assert marker not in data and marker_digest not in data

def run(label,program,args,input=None,success=True):
 r=subprocess.run([program,*args],env=env,cwd=world,input=input,capture_output=True,timeout=80);clean(r.stdout+r.stderr);steps.append({'label':label,'exit':r.returncode});assert (r.returncode==0)==success,label;return r
# Fresh self-contained synthetic distribution, using only explicitly mounted accepted artifacts.
def zipped(rows):
 b=io.BytesIO()
 with zipfile.ZipFile(b,'w',zipfile.ZIP_DEFLATED) as z:
  for n,v in rows:z.writestr(n,v)
 return b.getvalue()
source=zipped([('Cargo.toml',b'[workspace]\n'),('crates/vibe-cli/src/main.rs',b'fn main() {}')]);parts=[('vibe',pathlib.Path('/vibe').read_bytes()),('vibe-app-proxy',pathlib.Path('/proxy').read_bytes()),('vibe-index',pathlib.Path('/proxy').read_bytes())]
def h(b):return hashlib.sha256(b).hexdigest()
manifest={'schema_version':1,'product':'vibevm','repository':'vibevm/vibevm','version':'1.0.12','tag':'v1.0.12','source_commit':'1'*40,'target':'x86_64-unknown-linux-gnu','components':[{'name':n,'path':n,'size':len(v),'digest':'sha256:'+h(v)} for n,v in parts],'source_archive':{'path':'vibevm-source.zip','size':len(source),'digest':'sha256:'+h(source),'tree_oid':'2'*40}}
bundle=zipped(parts+[('DISTRIBUTION.json',json.dumps(manifest).encode()),('vibevm-source.zip',source),('LICENSE.md',b'synthetic'),('README.md',b'synthetic')]);bn='vibevm-1.0.12-x86_64-unknown-linux-gnu.zip';(kit/bn).write_bytes(bundle);profile=b'schema=1\nname="synthetic-recovery"\n[path]\nfix="never"\n';(kit/'profile-linux-auto.toml').write_bytes(profile);(kit/'SHA256SUMS.txt').write_text(h(bundle)+'  '+bn+'\n'+h(profile)+'  profile-linux-auto.toml\n');del bundle,parts
args=['--kit',str(kit),'--destination',str(install),'--settings',str(settings),'--no-path','--unattended'];run('mandatory-fresh-install','/installer',args)
cli=str(install/'opt/bin/vibe');foreign=settings/'foreign';foreign.write_bytes(b'KEEP-FOREIGN')
def query(label,args,input=None,success=True):
 r=run(label,cli,[*args,'--json'],input,success);return json.loads(r.stdout) if r.returncode==0 else None
secret=query('create-stdin-only',['secrets','create','--kind','api-key','--label','Synthetic recovery','--value-stdin'],marker);sid=secret['items'][0]['id'];revision=secret['items'][0]['current_revision']
a=query('add-reference',['ai','connections','add','--label','Synthetic recovery','--endpoint','http://127.0.0.1:32123/v1','--api-kind','openai-chat-completions','--default-model','synthetic','--secret-ref',sid]);cid=next(iter(a['items']))
# No default: selecting inherit-default in actual optional setup must genuinely fail.
key=settings/'secrets/index.toml';ai=settings/'secrets/ai/index.toml';before_key=key.read_bytes();before_ai=ai.read_bytes();baseline=query('active-before',['self','current']);binary_before=h(pathlib.Path(cli).read_bytes());claude_before=query('unselected-before',['ai','bindings','show','claude'])
def terminal(program,args,decline=False):
 master,slave=pty.openpty();original=termios.tcgetattr(slave)
 def session():os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
 child=subprocess.Popen([program,*args],env=env,cwd=world,stdin=slave,stdout=slave,stderr=slave,preexec_fn=session);data=b'';stage=0;last=0;answered=False;deadline=time.monotonic()+65
 while time.monotonic()<deadline:
  now=time.monotonic()
  if select.select([master],[],[],0.03)[0]:
   try:data+=os.read(master,65536)
   except OSError:break
  if stage==0 and b'Configure existing connection' in data and not termios.tcgetattr(slave)[3]&termios.ECHO:os.write(master,b' ');stage=1;last=now
  elif stage==1 and now-last>0.15:os.write(master,b'\x1b[B');stage=2;last=now
  elif stage==2 and now-last>0.15:os.write(master,b' ');stage=3;last=now
  elif stage==3 and now-last>0.15:os.write(master,b'\r');stage=4
  if not answered and b'Apply exactly this metadata selection' in data:os.write(master,b'\r' if decline else b'y\r');answered=True
  if child.poll() is not None:
   while select.select([master],[],[],0.02)[0]:
    try:data+=os.read(master,65536)
    except OSError:break
   break
 if child.poll() is None:child.kill();child.wait();raise AssertionError('fixture-owned PTY deadline')
 clean(data);restored=termios.tcgetattr(slave)==original;os.close(master);os.close(slave);assert restored;return child.returncode,data
code,data=terminal('/installer',[x for x in args if x!='--unattended']+['--configure-ai']);assert code==0 and b'"status":"failed"' in data;assert key.read_bytes()==before_key and ai.read_bytes()==before_ai;steps.append({'label':'actual-optional-failed','exit':code,'status_failed':True})
# Recover using exact installed CLI and same intact protected synthetic store.
setup=['ai','setup','--agent','pi','--agent','qwen','--mode','connection','--connection',cid]
preview=query('recovery-preview',setup);assert preview['applied']==False;assert ai.read_bytes()==before_ai and key.read_bytes()==before_key
# Actual attended default-No cancellation uses existing inherited mode; set default explicitly first.
current=query('connection-list',['ai','connections','list']);query('set-default',['ai','connections','default','set',cid,'--expect-store',str(current['store_revision'])]);cancel_before=ai.read_bytes();code,data=terminal(cli,['ai','setup'],True);assert code==0 and b'preview_id' in data;assert ai.read_bytes()==cancel_before and key.read_bytes()==before_key
fresh=query('fresh-recovery-preview',setup);query('confirmed-recovery',[*setup,'--apply-preview',fresh['preview']['preview_id']]);assert ai.read_bytes()!=cancel_before
for agent in ['pi','qwen']:
 row=query('fresh-read-'+agent,['ai','bindings','show',agent]);assert row['binding']['mode']=='connection' and row['binding']['connection']==cid
claude_after=query('unselected-after',['ai','bindings','show','claude'])
for field in ['harness','mode','connection','resolution','capability']:assert claude_after['binding'][field]==claude_before['binding'][field]
assert query('active-after',['self','current'])==baseline and h(pathlib.Path(cli).read_bytes())==binary_before and foreign.read_bytes()==b'KEEP-FOREIGN'
final=query('secret-id-after',['secrets','details',sid]);assert final['items'][0]['id']==sid and final['items'][0]['current_revision']==revision and key.read_bytes()==before_key
receipt={'exit':0,'steps':steps,'actual_optional_failed':True,'preview_cancel_preserve':True,'confirmed_selected_write':True,'fresh_process_reads':True,'unselected_unchanged':True,'secret_id_revision_unchanged':True,'mandatory_installation_unchanged':True,'foreign_file_unchanged':True,'marker_or_digest_in_output':False,'native_agent_provider_calls':False};(world/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))

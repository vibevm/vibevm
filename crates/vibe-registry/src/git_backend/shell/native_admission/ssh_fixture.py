# Isolated authenticated SSH fixture; server StrictModes=no for /tmp ancestry only.
import os,pathlib,subprocess,tempfile,socket,time,json,shlex,threading,select,hashlib,pwd
fixture_root=pathlib.Path(tempfile.mkdtemp(prefix='vibe-n3-test-'));fixture_root.chmod(0o700)
fixture_home=fixture_root/'home';fixture_home.mkdir(mode=0o700); EMPTY=fixture_root/'empty';EMPTY.write_text('')
fixture_user=pwd.getpwuid(os.getuid()).pw_name
ENV={'PATH':'/usr/bin:/bin','HOME':str(fixture_home),'XDG_CONFIG_HOME':str(fixture_home),'LC_ALL':'C','GIT_CONFIG_NOSYSTEM':'1','GIT_CONFIG_SYSTEM':str(EMPTY),'GIT_CONFIG_GLOBAL':str(EMPTY),'GIT_TERMINAL_PROMPT':'0'}
rows=[];children=[];logs=[]
def run(args,env=ENV,cwd=fixture_root):return subprocess.run(args,env=env,cwd=cwd,capture_output=True,timeout=20)
def ok(args,cwd=fixture_root):
 p=run(args,cwd=cwd)
 if p.returncode:raise RuntimeError('fixture setup failed: '+repr(args)+' '+p.stderr.decode())
 return p.stdout.decode().strip()
def write_script(path,body):path.write_text(body);path.chmod(0o700)
for name in ['host','wrong_host','identity','wrong_identity']:
 ok(['/usr/bin/ssh-keygen','-q','-t','ed25519','-N','','-f',str(fixture_root/name)])
(fixture_root/'authorized_keys').write_text('restrict '+(fixture_root/'identity.pub').read_text());(fixture_root/'authorized_keys').chmod(0o600)
work=fixture_root/'work';ok(['/usr/bin/git','init',str(work)]);(work/'payload').write_text('SSH exact source fixture\n')
(work/'vibe.toml').write_text("[package]\ngroup='org.fixture'\nname='demo'\nkind='feat'\nversion='1.0.0'\nmaterialization='in-place'\n")
if os.environ.get('VIBE_N3_INSTALL_FIXTURE')=='1':(work/'vibe.toml').write_text((work/'vibe.toml').read_text().replace('org.fixture','org.vibevm').replace("name='demo'","name='giant'"))
ok(['/usr/bin/git','add','.'],work);ok(['/usr/bin/git','-c','user.name=Fixture','-c','user.email=f@example.invalid','commit','-m','fixture'],work)
HEAD=ok(['/usr/bin/git','rev-parse','HEAD'],work);ok(['/usr/bin/git','tag','v1.0.0'],work)
(work/'payload').write_text('SSH updated fixture\n');(work/'vibe.toml').write_text((work/'vibe.toml').read_text().replace('1.0.0','1.1.0'));ok(['/usr/bin/git','add','.'],work);ok(['/usr/bin/git','-c','user.name=Fixture','-c','user.email=f@example.invalid','commit','-m','update'],work);ok(['/usr/bin/git','tag','v1.1.0'],work);UPDATED=ok(['/usr/bin/git','rev-parse','HEAD'],work)
bare=fixture_home/'repo.git';ok(['/usr/bin/git','clone','--bare',str(work),str(bare)])
# sshd invokes the account shell with -c. Explicit SetEnv HOME/BASH_ENV/ENV
# ensures its noninteractive startup sees only fixture paths, not real profiles.
forced=fixture_root/'serve.py'
write_script(forced,"#!/usr/bin/python3\nimport os,sys,shlex\nfrom pathlib import Path\nroot=Path("+repr(str(fixture_root))+")\n(root/'server-home-observed').write_text(os.environ.get('HOME',''))\nargs=shlex.split(os.environ.get('SSH_ORIGINAL_COMMAND',''))\nif args not in [['git-upload-pack',str(root/'home/repo.git')],['git-upload-pack','repo.git'],['git-upload-pack',str(root/'home/repo.git/org.fixture.demo.git')]]:sys.exit(91)\nos.execve('/usr/bin/git-upload-pack',['git-upload-pack',str(root/'home/repo.git')],"+repr(ENV)+")\n")
sock=socket.socket();sock.bind(('127.0.0.1',0));PORT=sock.getsockname()[1];sock.close()
def server(host,label):
 cfg=fixture_root/('sshd-'+label+'.config');log=fixture_root/('sshd-'+label+'.log')
 cfg.write_text(f'''Port {PORT}
ListenAddress {host}
HostKey {fixture_root/'host'}
PidFile {fixture_root/('pid-'+label)}
AuthorizedKeysFile {fixture_root/'authorized_keys'}
StrictModes no
UsePAM no
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
AuthenticationMethods publickey
PermitRootLogin no
AllowUsers {fixture_user}
PermitUserEnvironment no
PermitUserRC no
AllowAgentForwarding no
AllowTcpForwarding no
X11Forwarding no
PermitTunnel no
PermitTTY no
PrintMotd no
PrintLastLog no
UseDNS no
LogLevel VERBOSE
PerSourcePenalties no
ForceCommand {forced}
SetEnv HOME={fixture_home} BASH_ENV={EMPTY} ENV={EMPTY}
''')
 validate=run(['/usr/sbin/sshd','-t','-f',str(cfg)])
 if validate.returncode:raise RuntimeError('isolated sshd validation failed: '+validate.stderr.decode())
 f=log.open('wb');logs.append(f)
 child=subprocess.Popen(['/usr/sbin/sshd','-D','-e','-f',str(cfg)],env=ENV,stdin=subprocess.DEVNULL,stdout=f,stderr=f);children.append(child)
 for _ in range(50):
  if child.poll() is not None:raise RuntimeError('isolated sshd exited '+str(child.returncode)+': '+log.read_text())
  try:
   with socket.create_connection((host,PORT),timeout=.1):return
  except OSError:time.sleep(.02)
 raise RuntimeError('isolated sshd did not listen')
def connections(label):
 return (fixture_root/('sshd-'+label+'.log')).read_text().count('Connection from ')
def hostfile(name,key):
 key=' '.join((fixture_root/(key+'.pub')).read_text().split()[:2]);p=fixture_root/name
 p.write_text(f'[127.0.0.1]:{PORT} {key}\n[127.0.0.2]:{PORT} {key}\n');return p
known=hostfile('known_hosts','host');wrong=hostfile('wrong_known_hosts','wrong_host')
def ssh_config(name,identity='identity',hosts=known,extra=''):
 p=fixture_root/name;p.write_text(f'''Host *
 User {fixture_user}
 Port {PORT}
 IdentityFile {fixture_root/identity}
 IdentitiesOnly yes
 IdentityAgent none
 UserKnownHostsFile {hosts}
 GlobalKnownHostsFile /dev/null
 StrictHostKeyChecking yes
 BatchMode yes
 PasswordAuthentication no
 KbdInteractiveAuthentication no
 CanonicalizeHostname no
 PermitLocalCommand no
 ForwardAgent no
 ForwardX11 no
 ClearAllForwardings yes
 ControlMaster no
 ControlPath none
 UpdateHostKeys no
 RequestTTY no
 ConnectTimeout 3
{extra}
''');return p
controlled=ssh_config('controlled.config',extra=' ProxyCommand none\n ProxyJump none')
legacy_alias=ssh_config('legacy-alias.config',extra=' HostName 127.0.0.2')
marker=fixture_root/'PROXY_EXECUTED';proxy=fixture_root/'proxy.py'
write_script(proxy,"#!/usr/bin/python3\nimport socket,select,os\nfrom pathlib import Path\nPath("+repr(str(marker))+").write_text('executed')\ns=socket.create_connection(('127.0.0.2',"+str(PORT)+"))\nwhile True:\n ready,_,_=select.select([0,s],[],[],10)\n for r in ready:\n  data=os.read(0,65536) if r==0 else s.recv(65536)\n  if not data:raise SystemExit(0)\n  s.sendall(data) if r==0 else os.write(1,data)\n")
legacy_proxy=ssh_config('legacy-proxy.config',extra=' ProxyCommand '+str(proxy))
bad_host=ssh_config('bad-host.config',hosts=wrong,extra=' ProxyCommand none\n ProxyJump none')
bad_auth=ssh_config('bad-auth.config',identity='wrong_identity',extra=' ProxyCommand none\n ProxyJump none')
def git_env(config):
 wrapper=fixture_root/(config.stem+'-ssh')
 write_script(wrapper,"#!/usr/bin/python3\nimport os,sys\nos.execv('/usr/bin/ssh',['/usr/bin/ssh','-F',"+repr(str(config))+"]+sys.argv[1:])\n")
 return dict(ENV,GIT_SSH=str(wrapper),GIT_SSH_VARIANT='ssh',GIT_ALLOW_PROTOCOL='ssh')
url=f'ssh://{fixture_user}@127.0.0.1:{PORT}{bare}'
def probe(name,args,config,cwd=fixture_root):
 before=[connections(x) for x in ['a','b']];p=run(['/usr/bin/git',*args],env=git_env(config),cwd=cwd)
 time.sleep(.05)
 row={'name':name,'exit':p.returncode,'A_connections':connections('a')-before[0],'B_connections':connections('b')-before[1],'stdout':p.stdout.decode(),'stderr':p.stderr.decode()};rows.append(row);return row
try:
 server('127.0.0.1','a');server('127.0.0.2','b');time.sleep(.2)
 # A private explicitly addressed agent, never the operator's agent.
 agent_socket=fixture_root/'agent.sock';agent_log=(fixture_root/'agent.log').open('wb');logs.append(agent_log)
 agent=subprocess.Popen(['/usr/bin/ssh-agent','-D','-a',str(agent_socket)],env=ENV,stdin=subprocess.DEVNULL,stdout=agent_log,stderr=agent_log);children.append(agent)
 for _ in range(50):
  if agent.poll() is not None:raise RuntimeError('owned ssh-agent exited')
  if agent_socket.exists():break
  time.sleep(.02)
 added=run(['/usr/bin/ssh-add','-q',str(fixture_root/'identity')],env=dict(ENV,SSH_AUTH_SOCK=str(agent_socket)))
 if added.returncode:raise RuntimeError('loading owned fixture agent failed')
 controlled_wrapper=git_env(controlled)['GIT_SSH']
 alias_wrapper=git_env(legacy_alias)['GIT_SSH'];proxy_wrapper=git_env(legacy_proxy)['GIT_SSH']
 print(json.dumps({'root':str(fixture_root),'home':str(fixture_home),'user':fixture_user,'port':PORT,'url':url,'scp':fixture_user+'@127.0.0.1:repo.git','head':HEAD,'updated':UPDATED,'key':str(fixture_root/'identity'),'wrong_key':str(fixture_root/'wrong_identity'),'known_hosts':str(known),'wrong_hosts':str(wrong),'agent_socket':str(agent_socket),'controlled_wrapper':controlled_wrapper,'alias_wrapper':alias_wrapper,'proxy_wrapper':proxy_wrapper,'proxy_marker':str(marker)}),flush=True)
 import sys
 sys.stdin.read()
finally:
 for child in children:
  if child.poll() is None:child.terminate()
 for child in children:
  try:child.wait(timeout=5)
  except subprocess.TimeoutExpired:child.kill();child.wait()
 for f in logs:f.close()

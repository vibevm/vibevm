"""Linux-only counters over owned test subprocesses; fixture setup is explicit."""
import ast,hashlib,json,os,re,subprocess,sys
from pathlib import Path
E=Path(__file__).parent
label=sys.argv[1];argv=sys.argv[2:];assert argv
home=E/(label+'-observer-home');(home/'.vibe/cache').mkdir(parents=True,exist_ok=True)
env=dict(PATH='/usr/bin:/bin',LANG='C.UTF-8',HOME=str(home),VIBE_SETTINGS=str(home/'.vibe'),VIBE_NO_DEFAULT_REGISTRY='1',VIBE_NO_PROGRESS='1')
trace=E/(label+'.strace')
command=['strace','-f','-yy','-s','16384','-o',str(trace),'-e','trace=%file,%network,%process,write,writev,pwrite64,pwritev,pwritev2,ftruncate,fchmod,fchown,fsync,fdatasync']+argv
r=subprocess.run(command,cwd='/tmp',env=env,capture_output=True,text=True,timeout=180)
(E/(label+'.stdout.log')).write_text(r.stdout);(E/(label+'.stderr.log')).write_text(r.stderr)
(E/(label+'-command.json')).write_text(json.dumps(dict(argv=command,cwd='/tmp',exit=r.returncode,env={k:env[k] for k in ['HOME','VIBE_SETTINGS','VIBE_NO_DEFAULT_REGISTRY','VIBE_NO_PROGRESS']},binary_sha256=hashlib.sha256(Path(argv[0]).read_bytes()).hexdigest()),indent=2)+'\n')
assert r.returncode==0,(label,r.returncode,r.stderr[-2000:],r.stdout[-2000:])
scopes={};active={};buffers={};rows=[];child_scope={};diagnostics=[]
net=re.compile(r'\b(socket|socketpair|connect|sendto|sendmsg|sendmmsg|bind|listen|accept|accept4|recvfrom|recvmsg|recvmmsg|shutdown|getsockname|getpeername|getsockopt|setsockopt)\(')
mut=re.compile(r'\b(mkdir|mkdirat|rename|renameat|renameat2|unlink|unlinkat|link|linkat|symlink|symlinkat|creat|truncate|ftruncate|chmod|fchmod|fchmodat|fchmodat2|chown|fchown|fchownat|mknod|mknodat|utime|utimes|utimensat|futimesat|fsync|fdatasync)\(')
quotes=re.compile(r'"(?:[^"\\]|\\.)*"')
def strings(line):
 vals=[]
 for token in quotes.findall(line):
  try: vals.append(ast.literal_eval(token))
  except (ValueError,SyntaxError):pass
 return vals
for line in trace.read_text().splitlines():
 m=re.match(r'\s*(\d+)\s+(.*)',line)
 if not m:continue
 pid,call=m.groups()
 if 'execve(' in call:
  vals=strings(call)
  if vals and Path(vals[0]).name=='vibe':
   name='surface-process-'+pid;child_scope[pid]=name;scopes[name]={'network_attempts':0,'filesystem_mutation_attempts':0,'process_spawn_attempts':0,'kind':'whole CLI/MCP child including startup'}
 clone=re.search(r'\b(?:clone|clone3|fork|vfork)\(.*=\s+(\d+)\s*$',call)
 if clone and pid in child_scope:child_scope[clone.group(1)]=child_scope[pid]
 current=active.get(pid) or child_scope.get(pid)
 network=bool(net.search(call));mutation=bool(mut.search(call)) or bool(re.search(r'\bopen(?:at|at2)?\(.*O_(?:WRONLY|RDWR|CREAT|TRUNC|TMPFILE)',call))
 # Writes to pipe/socket/stdout/stderr are transport. Other named absolute
 # file descriptors remain mutation attempts even if the kernel refuses them.
 if re.search(r'\b(?:write|writev|pwrite64|pwritev|pwritev2)\(\d+</',call):mutation=True
 spawn=bool(re.search(r'\b(?:execve|execveat|clone|clone3|fork|vfork)\(',call))
 if current:
  row=scopes[current];row['network_attempts']+=network;row['filesystem_mutation_attempts']+=mutation;row['process_spawn_attempts']+=spawn
  if network or mutation:diagnostics.append({'scope':current,'line':line})
 if re.search(r'\bwrite(?:v)?\(2(?:<[^>]*>)?,',call):
  text=''.join(strings(call));buffers[pid]=buffers.get(pid,'')+text
  while '\n' in buffers[pid]:
   part,buffers[pid]=buffers[pid].split('\n',1)
   begin=re.search(r'E2_QUERY_BEGIN (\S+)',part);end=re.search(r'E2_QUERY_END (\S+)',part)
   if begin:
    name=pid+':'+begin.group(1)+':'+str(len(rows));assert pid not in active,(label,'nested interval',part)
    active[pid]=name;scopes[name]={'network_attempts':0,'filesystem_mutation_attempts':0,'process_spawn_attempts':0,'kind':'explicit query interval'}
   if end:
    name=active.pop(pid);assert end.group(1) in name,(name,part);rows.append(name)
assert not active,(label,'unterminated query intervals',active)
assert rows,(label,'no measured query markers')
report=dict(label=label,linux_only=True,complete_query_intervals=len(rows),whole_surface_processes=len(set(child_scope.values())),scopes=scopes,mutation_or_network_attempts=diagnostics,trace_sha256=hashlib.sha256(trace.read_bytes()).hexdigest(),runtime_exit=r.returncode)
(E/(label+'-counters.json')).write_text(json.dumps(report,indent=2)+'\n')
assert not diagnostics,(label,diagnostics[:5])
print(json.dumps(dict(label=label,complete_query_intervals=len(rows),whole_surface_processes=len(set(child_scope.values())),network_attempts=0,filesystem_mutation_attempts=0)))

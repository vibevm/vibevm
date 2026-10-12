"""Linux-only counters over owned test subprocesses; fixture setup is explicit."""
import ast,hashlib,json,os,re,subprocess,sys
from pathlib import Path
E=Path(__file__).parent
label=sys.argv[1];argv=sys.argv[2:];assert argv
home=E/(label+'-observer-home');(home/'.vibe/cache').mkdir(parents=True,exist_ok=True)
env=dict(PATH='/usr/bin:/bin',LANG='C.UTF-8',HOME=str(home),VIBE_SETTINGS=str(home/'.vibe'),VIBE_NO_DEFAULT_REGISTRY='1',VIBE_NO_PROGRESS='1')
for key,relative in [('XDG_CONFIG_HOME','.config'),('XDG_CACHE_HOME','.cache'),('XDG_STATE_HOME','.local/state'),('XDG_DATA_HOME','.local/share'),('XDG_RUNTIME_DIR','runtime')]:
 target=home/relative;target.mkdir(parents=True,exist_ok=True);target.chmod(0o700);env[key]=str(target)
trace=E/(label+'.strace')
command=['strace','-f','-yy','-s','16384','-o',str(trace),'-e','trace=%file,%network,%desc,%memory,%process']+argv
replay_of=None
if argv[0]=='--replay':
 from types import SimpleNamespace
 replay_of=argv[1];prior=json.loads((E/(replay_of+'-command.json')).read_text())
 trace=E/(replay_of+'.strace');command=prior['argv'];binary=command[command.index('-e')+2]
 r=SimpleNamespace(returncode=prior['exit'],stdout=(E/(replay_of+'.stdout.log')).read_text(),stderr=(E/(replay_of+'.stderr.log')).read_text())
else:
 binary=argv[0];r=subprocess.run(command,cwd='/tmp',env=env,capture_output=True,text=True,timeout=180)
(E/(label+'.stdout.log')).write_text(r.stdout);(E/(label+'.stderr.log')).write_text(r.stderr)
(E/(label+'-command.json')).write_text(json.dumps(dict(argv=command,cwd='/tmp',exit=r.returncode,env=prior['env'] if replay_of else {k:env[k] for k in ['HOME','VIBE_SETTINGS','VIBE_NO_DEFAULT_REGISTRY','VIBE_NO_PROGRESS','XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_STATE_HOME','XDG_DATA_HOME','XDG_RUNTIME_DIR']},binary_sha256=hashlib.sha256(Path(binary).read_bytes()).hexdigest(),analysis_only_replay_of=replay_of),indent=2)+'\n')
assert r.returncode==0,(label,r.returncode,r.stderr[-2000:],r.stdout[-2000:])
scopes={};active={};buffers={};rows=[];child_scope={};diagnostics=[];allowances=[];mappings={}
net=re.compile(r'\b(socket|socketpair|connect|sendto|sendmsg|sendmmsg|bind|listen|accept|accept4|recvfrom|recvmsg|recvmmsg|shutdown|getsockname|getpeername|getsockopt|setsockopt)\(')
mut=re.compile(r'\b(mkdir|mkdirat|rename|renameat|renameat2|unlink|unlinkat|link|linkat|symlink|symlinkat|creat|truncate|ftruncate|chmod|fchmod|fchmodat|fchmodat2|chown|fchown|fchownat|mknod|mknodat|utime|utimes|utimensat|futimesat|fsync|fdatasync)\(')
quotes=re.compile(r'"(?:[^"\\]|\\.)*"')
def strings(line):
 vals=[]
 for token in quotes.findall(line):
  try: vals.append(ast.literal_eval(token))
  except (ValueError,SyntaxError):pass
 return vals
def fd_kind(argument):
 m=re.match(r'\s*(-?\d+)(?:<([^>]+)>)?',argument)
 if not m:return 'unknown'
 fd,path=m.groups()
 if path and path.startswith('pipe:'):return 'pipe'
 if path and (path.startswith('UNIX-') or path.startswith('TCP') or path.startswith('UDP') or path.startswith('socket:')):return 'socket'
 if path and path.startswith('anon_inode:'):return 'anonymous-control'
 if path and path.startswith('/'):return 'file'
 return 'unknown'
def intersects(addr,length,start,size):return addr<start+size and start<addr+length
def remove_range(rows,addr,size):
 result=[]
 for start,length,file_backed in rows:
  end=start+length
  if not intersects(addr,size,start,length):result.append((start,length,file_backed));continue
  if start<addr:result.append((start,addr-start,file_backed))
  if addr+size<end:result.append((addr+size,end-addr-size,file_backed))
 return result
for line in trace.read_text().splitlines():
 m=re.match(r'\s*(\d+)\s+(.*)',line)
 if not m:continue
 pid,call=m.groups()
 if 'execve(' in call:
  mappings[pid]=[]
  vals=strings(call)
  if vals and Path(vals[0]).name=='vibe':
   name='surface-process-'+pid;child_scope[pid]=name;scopes[name]={'network_attempts':0,'filesystem_mutation_attempts':0,'process_spawn_attempts':0,'kind':'CLI/MCP exec-to-exit including post-exec startup'}
 clone=re.search(r'\b(?:clone|clone3|fork|vfork)\(.*=\s+(\d+)\s*$',call)
 if clone:
  child=clone.group(1)
  if pid in child_scope:child_scope[child]=child_scope[pid]
  mappings[child]=mappings.setdefault(pid,[]) if 'CLONE_VM' in call else list(mappings.get(pid,[]))
 current=active.get(pid) or child_scope.get(pid)
 network=bool(net.search(call));mutation=bool(mut.search(call)) or bool(re.search(r'\bopen(?:at|at2)?\(.*O_(?:WRONLY|RDWR|CREAT|TRUNC|TMPFILE)',call))
 socket_io=re.match(r'(?:read|readv|pread64|preadv|preadv2|write|writev|pwrite64|pwritev|pwritev2|sendfile|sendfile64|ioctl)\(([^,]+)',call)
 if socket_io and fd_kind(socket_io.group(1))=='socket':network=True
 reason='path/descriptor mutation' if mutation else None
 descriptor=re.match(r'(write|writev|pwrite64|pwritev|pwritev2|sendfile|sendfile64|fallocate|fsetxattr|fremovexattr|flock|ioctl)\(([^,]+)',call)
 if descriptor:
  operation,arg=descriptor.groups();kind=fd_kind(arg)
  if operation not in ['ioctl','flock'] or operation=='flock' or kind not in ['pipe','socket','anonymous-control']:
   if kind in ['file','unknown'] or operation=='flock':mutation=True;reason='file/unknown descriptor mutation attempt'
   elif current:allowances.append({'scope':current,'reason':'decoded transport/anonymous-control descriptor','line':line})
 if re.match(r'fcntl(?:64)?\(',call) and re.search(r'F_(?:OFD_)?SETLK(?:W)?',call):mutation=True;reason='file lock attempt'
 for operation,index in [('copy_file_range',2),('splice',2)]:
  if call.startswith(operation+'('):
   parts=call.split('(',1)[1].split(',')
   if fd_kind(parts[index]) in ['file','unknown']:mutation=True;reason='descriptor transfer to file/unknown destination'
 mapped=re.match(r'mmap(?:2)?\((.*)\)\s*=\s*(\S+)',call)
 if mapped:
  args,result=mapped.groups();parts=args.split(',')
  if len(parts)>=6:
   shared='MAP_SHARED' in parts[3];anonymous='MAP_ANONYMOUS' in parts[3] or parts[4].strip()=='-1';file_backed=not anonymous and fd_kind(parts[4]) not in ['anonymous-control','pipe','socket']
   if shared and file_backed and 'PROT_WRITE' in parts[2]:mutation=True;reason='attempted shared writable file mapping'
   elif current:allowances.append({'scope':current,'reason':'anonymous/private mapping or nonwritable shared mapping','line':line})
   if result.startswith('0x'):
    address=int(result,16);length=int(parts[1].strip(),0)
    mappings[pid]=remove_range(mappings.get(pid,[]),address,length)+[(address,length,shared and file_backed)]
 protected=re.match(r'(mprotect|pkey_mprotect|msync|mremap)\((0x[0-9a-f]+),\s*(\d+),\s*(.*)',call)
 if protected:
  operation,addr,size,rest=protected.groups();shared_file=any(is_file and intersects(int(addr,16),int(size),start,length) for start,length,is_file in mappings.get(pid,[]))
  if shared_file and (operation in ['msync','mremap'] or 'PROT_WRITE' in rest):mutation=True;reason='shared file mapping protection/sync/remap attempt'
  elif current:allowances.append({'scope':current,'reason':'private/anonymous or nonwritable protection operation','line':line})
 remapped=re.match(r'mremap\((0x[0-9a-f]+),\s*(\d+),\s*(\d+),.*\)\s*=\s*(0x[0-9a-f]+)',call)
 if remapped:
  old,size,new_size,new=remapped.groups();old=int(old,16);size=int(size);new=int(new,16);new_size=int(new_size)
  file_backed=any(flag and intersects(old,size,start,length) for start,length,flag in mappings.get(pid,[]))
  previous=mappings.get(pid,[]) if 'MREMAP_DONTUNMAP' in call else remove_range(mappings.get(pid,[]),old,size)
  mappings[pid]=remove_range(previous,new,new_size)+[(new,new_size,file_backed)]
 unmapped=re.match(r'munmap\((0x[0-9a-f]+),\s*(\d+)\)\s*=\s*0',call)
 if unmapped:
  addr,size=unmapped.groups();mappings[pid]=remove_range(mappings.get(pid,[]),int(addr,16),int(size))
 spawn=bool(re.search(r'\b(?:execve|execveat|clone|clone3|fork|vfork)\(',call))
 if current:
  row=scopes[current];row['network_attempts']+=network;row['filesystem_mutation_attempts']+=mutation;row['process_spawn_attempts']+=spawn
  if network or mutation:diagnostics.append({'scope':current,'reason':'network attempt' if network else reason,'line':line})
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
report=dict(label=label,linux_only=True,complete_query_intervals=len(rows),whole_surface_processes=len(set(child_scope.values())),scopes=scopes,mutation_or_network_attempts=diagnostics,trace_sha256=hashlib.sha256(trace.read_bytes()).hexdigest(),runtime_exit=r.returncode,allowances=allowances,oracle_scope='pathname + descriptor + memory/mapping + network attempts; failed calls included')
(E/(label+'-counters.json')).write_text(json.dumps(report,indent=2)+'\n')
if label.startswith('observer-positive-control'):
 assert any('pwrite64(' in x['line'] for x in diagnostics),diagnostics
 assert any('pwrite64(' in x['line'] and '= -1' in x['line'] for x in diagnostics),diagnostics
 assert any('shared writable file mapping'==x['reason'].removeprefix('attempted ') for x in diagnostics),diagnostics
 assert any('shared file mapping protection' in (x['reason'] or '') for x in diagnostics),diagnostics
 assert any('connect(' in x['line'] for x in diagnostics),diagnostics
 print(json.dumps({'positive_control_detected_attempts':len(diagnostics),'failed_fd_attempt_detected':True,'shared_mapping_detected':True,'loopback_connect_detected':True}));sys.exit(0)
assert not diagnostics,(label,diagnostics[:5])
print(json.dumps(dict(label=label,complete_query_intervals=len(rows),whole_surface_processes=len(set(child_scope.values())),network_attempts=0,filesystem_mutation_attempts=0)))

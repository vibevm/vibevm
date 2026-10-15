from pathlib import Path
import re,json,hashlib
root=Path(__file__).resolve().parent
trace=root/'V2.trace'
active={}; completed=[]
mutations={'creat','mkdir','mkdirat','unlink','unlinkat','rmdir','rename','renameat','renameat2','link','linkat','symlink','symlinkat','truncate','ftruncate','chmod','fchmod','fchmodat','chown','fchown','lchown','fchownat','utime','utimes','utimensat','mknod','mknodat','mount','umount2','fsync','fdatasync','pwrite64','pwritev','pwritev2'}
process={'clone','clone3','fork','vfork','execve','execveat'}
network={'socket','socketpair','connect','bind','listen','accept','accept4','sendto','sendmsg','sendmmsg','recvfrom','recvmsg','recvmmsg','shutdown','getsockopt','setsockopt'}
for line_no,line in enumerate(trace.open(),1):
 m=re.match(r'(\d+)\s+(.*)',line)
 if not m:continue
 pid,body=m.groups()
 if re.match(r'write\(2, "B2_QUERY_BEGIN ',body):
  assert pid not in active
  active[pid]={'tid':pid,'begin_line':line_no,'syscalls':{},'effects':[]};continue
 if re.match(r'write\(2, "B2_QUERY_END ',body):
  assert pid in active
  row=active.pop(pid);row['end_line']=line_no;completed.append(row);continue
 if pid not in active:continue
 m=re.match(r'(\w+)\(',body)
 if not m:continue # resumed syscalls are already counted at their initial line
 call=m[1];row=active[pid];row['syscalls'][call]=row['syscalls'].get(call,0)+1
 effect=None
 if call in mutations:effect='filesystem mutation'
 if call in process:effect='process'
 if call in network:effect='network'
 if call in {'open','openat','openat2'} and re.search(r'O_(WRONLY|RDWR|CREAT|TRUNC|APPEND|TMPFILE)',body):effect='write-open'
 if call in {'write','writev'} and not re.match(r'write(?:v)?\(2,',body):effect='write'
 if call=='flock' or call=='fcntl' and re.search(r'F_(SETLK|SETLKW|OFD_SETLK|OFD_SETLKW)',body):effect='lock'
 if effect:row['effects'].append({'line':line_no,'category':effect,'syscall':body})
assert not active,active
assert completed
result={'trace_sha256':hashlib.sha256(trace.read_bytes()).hexdigest(),'query_intervals':len(completed),'effects_count':sum(len(x['effects']) for x in completed),'intervals':completed,'scope':'Linux selected traced syscall families, actual child query thread intervals only; marker writes to stderr excluded; setup, compiler, inert child launch and teardown outside intervals. No cross-platform or hostile-ABA guarantee.'}
(root/'worker-query-census.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k!='intervals'}))
assert result['effects_count']==0

import pathlib,tempfile,subprocess,os,json,hashlib,re
out=pathlib.Path(__file__).parent
binary=pathlib.Path('/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/vibe')
def census(root):
    if not root.exists(): return None
    return {str(p.relative_to(root)): ('directory' if p.is_dir() else hashlib.sha256(p.read_bytes()).hexdigest()) for p in sorted(root.rglob('*'))}
with tempfile.TemporaryDirectory(dir=out/'tmp',prefix='cli-syscalls-') as temp:
    base=pathlib.Path(temp);root=base/'project';root.mkdir();(root/'skill').mkdir()
    (root/'vibe.toml').write_text("[package]\ngroup='org.example'\nname='fixture'\nkind='tool'\nversion='1.0.0'\nauthors=['Fixture']\nlicense='MIT'\ndescription='Fixture'\nkeywords=['fixture']\n[[skill]]\nname='demo'\npath='skill'\nagents=['claude']\n")
    (root/'skill/SKILL.md').write_text('body');(root/'skill/reference.txt').write_text('reference')
    roots={name:base/name for name in ['project','settings','cache','home','receipts','locks','scratch']}
    roots['scratch'].mkdir()
    env=os.environ.copy();env.update(VIBE_SETTINGS=str(roots['settings']),VIBE_REGISTRY_CACHE=str(roots['cache']),TMPDIR=str(roots['scratch']))
    trace=out/'worker-cli-syscalls.trace'
    argv=['/usr/bin/strace','-f','-s','16384','-o',str(trace),'-e','trace=%file,%process,%network,write,writev,flock,fcntl',str(binary),'--offline','adapter-plan','--root',str(root),'--kind','skills','--prepare']
    child=subprocess.Popen(argv,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    lines=[];lines.append(child.stdout.readline());session_env=json.loads(lines[0]);session=json.loads(session_env['body']['payload_json'])
    assert session['body']['kind']=='session' and session['body']['preparation']=='ready'
    before={name:census(p) for name,p in roots.items()}
    req={'schema':1,'operation_id':'rust:vibe-orchestrator:adapter-plan','domain_owner':'rust:vibe-orchestrator','source_id':'prepared-adapter-plan/1','evidence_id':None,'body':{'stage':'input','format_id':'adapter-plan-request','payload_json':json.dumps({'schema':1,'context_id':session['context_id'],'target_id':session['target_id'],'command':{'action':'plan','capture_id':session['capture_id']}})}}
    for _ in range(2):
        child.stdin.write(json.dumps(req)+'\n');child.stdin.flush();lines.append(child.stdout.readline())
    after={name:census(p) for name,p in roots.items()};assert before==after
    child.stdin.close();tail=child.stdout.read();stderr=child.stderr.read();exitcode=child.wait();assert exitcode==0 and not tail
    replies=[json.loads(line) for line in lines];assert replies[1]==replies[2]
    plan=json.loads(replies[1]['body']['payload_json']);assert plan['body']['rows']
    raw=trace.read_text().splitlines();start=next(i for i,l in enumerate(raw) if 'write(1,' in l and 'adapter-plan-result' in l and 'session' in l)
    query=raw[start+1:];mutations=[];process=[];network=[];locks=[]
    for line in query:
        if re.search(r'\b(?:mkdir|mkdirat|unlink|unlinkat|rename|renameat|renameat2|link|linkat|symlink|symlinkat|chmod|fchmod|fchmodat|chown|truncate|utime|utimes|utimensat)\(',line) or (re.search(r'\bopen(?:at|at2)?\(',line) and re.search(r'O_(?:WRONLY|RDWR|CREAT|TRUNC|APPEND)',line)): mutations.append(line)
        if re.search(r'\b(?:clone|clone3|fork|vfork|execve|execveat)\(',line):process.append(line)
        if re.search(r'\b(?:socket|connect|sendto|sendmsg|bind|listen|accept|accept4)\(',line):network.append(line)
        if 'flock(' in line or re.search(r'fcntl\(.*F_(?:SETLK|SETLKW)',line):locks.append(line)
    result={'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'argv':argv,'exit':exitcode,'query_start':'first Session write completed','query_trace_lines':len(query),'rows':len(plan['body']['rows']),'effects':{'mutation_attempts':len(mutations),'process_attempts':len(process),'network_attempts':len(network),'lock_attempts':len(locks)},'violations':mutations+process+network+locks,'before':before,'after':after,'stdout':lines,'stderr':stderr,'scope':'actual Linux child only; syscall observation, not cross-platform proof'}
    (out/'worker-cli-syscalls.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['binary_sha256','exit','query_trace_lines','rows','effects']}))
    assert not result['violations']

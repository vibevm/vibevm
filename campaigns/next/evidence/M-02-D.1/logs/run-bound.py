import json, os, pathlib, re, subprocess, sys, hashlib
home=pathlib.Path('/fast/git/v/research/2026-10-12-vibevm-first-agent-start/M02-D1')
name=sys.argv[1]
cmd=['/home/olegchir/.cargo/bin/cargo','+nightly',*sys.argv[2:]]
env=os.environ.copy();env.update(RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR='/fast/git/v/.build/c4-first-agent-target')
with (home/(name+'.log')).open('wb') as log: result=subprocess.run(cmd,cwd='/tmp',env=env,stdout=log,stderr=subprocess.STDOUT)
raw=(home/(name+'.log')).read_bytes()
counts=re.findall(rb'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored',raw)
record={'name':name,'command':cmd,'cwd':'/tmp','env':{k:env[k] for k in ['RUSTUP_AUTO_INSTALL','CARGO_NET_OFFLINE','CARGO_TARGET_DIR']},'exit':result.returncode,'test_summaries':[[x.decode() for x in row] for row in counts],'log_sha256':hashlib.sha256(raw).hexdigest()}
with (home/'RUNS.jsonl').open('a') as out:out.write(json.dumps(record)+'\n')
print(json.dumps(record));sys.exit(result.returncode)

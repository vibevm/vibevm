import hashlib,json,os,pathlib,subprocess,sys,time
home=pathlib.Path(__file__).parent
label,package,selector=sys.argv[1:]
command=['/home/olegchir/.cargo/bin/cargo','+nightly','test','--manifest-path','/fast/git/v/vibevm-next/Cargo.toml','--locked','--offline','-p',package,'--lib',selector,'--','--nocapture']
if not selector.endswith('::'): command.append('--exact')
env=os.environ.copy();env.update(RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR='/fast/git/v/.build/c4-first-agent-target')
start=time.time()
with (home/(label+'.log')).open('w') as output:
 result=subprocess.run(command,cwd='/tmp',env=env,stdout=output,stderr=subprocess.STDOUT)
record={'label':label,'command':command,'cwd':'/tmp','environment':{k:env[k] for k in ['RUSTUP_AUTO_INSTALL','CARGO_NET_OFFLINE','CARGO_TARGET_DIR']},'exit':result.returncode,'seconds':time.time()-start,'log_sha256':hashlib.sha256((home/(label+'.log')).read_bytes()).hexdigest()}
p=home/'COMMANDS.json'; rows=json.loads(p.read_text()) if p.exists() else [];rows.append(record);p.write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps(record)); print('\n'.join(line for line in (home/(label+'.log')).read_text().splitlines() if line.startswith(('running ','test ','BOUND_VECTOR','error:','thread '))))
sys.exit(result.returncode)

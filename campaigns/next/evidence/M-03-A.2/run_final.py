from pathlib import Path
import os,subprocess,json
out=Path('/fast/git/v/research/2026-10-12-vibevm-first-agent-start/M03-A2')
cargo=['/home/olegchir/.cargo/bin/cargo','+nightly']
base=['--manifest-path','/fast/git/v/vibevm-next/Cargo.toml','--locked','--offline']
env=os.environ.copy();env.update(RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR='/fast/git/v/.build/c4-first-agent-target')
checks=[('orchestrator-final',cargo+['test']+base+['-p','vibe-orchestrator','--lib','change::m_03_a_2_cases::','--','--nocapture']),('cli-policy',cargo+['test']+base+['-p','vibe-cli','--bin','vibe','command_progress::tests::change_prepare_and_explicit_apply_use_early_policy','--','--exact','--nocapture']),('cli-build',cargo+['build']+base+['-p','vibe-cli','--bin','vibe']),('fullfmt',cargo+['fmt','--manifest-path','/fast/git/v/vibevm-next/Cargo.toml','--all','--','--check'])]
records=[]
for name,argv in checks:
 with (out/(name+'.log')).open('wb') as log:r=subprocess.run(argv,cwd='/tmp',env=env,stdout=log,stderr=subprocess.STDOUT)
 (out/(name+'.exit')).write_text(str(r.returncode)+'\n');records.append({'name':name,'argv':argv,'cwd':'/tmp','exit':r.returncode,'environment':{k:env[k] for k in ['RUSTUP_AUTO_INSTALL','CARGO_NET_OFFLINE','CARGO_TARGET_DIR']}})
 (out/'COMMANDS-FINAL.json').write_text(json.dumps(records,indent=2)+'\n')
 print(name,r.returncode,flush=True)
 if r.returncode and name!='fullfmt':break

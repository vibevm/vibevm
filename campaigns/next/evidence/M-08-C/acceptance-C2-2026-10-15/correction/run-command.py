import sys,json,hashlib,subprocess,time,os
from pathlib import Path
r=Path(__file__).resolve().parent; repo=Path('/fast/git/v/vibevm-next')
label,*argv=sys.argv[1:]; paths=json.loads((r/'source-baseline.json').read_text())['sha256']
def hashes(): return {n:hashlib.sha256((repo/n).read_bytes()).hexdigest() for n in paths}
before=hashes(); start=time.monotonic()
env=os.environ.copy(); env.update(CARGO_TARGET_DIR='/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target',TMPDIR=str(r/'tmp'),M08C2_EVIDENCE_DIR=str(r/'censuses'))
(r/'censuses').mkdir(exist_ok=True)
with (r/(label+'.log')).open('wb') as f: p=subprocess.run(argv,cwd=repo,env=env,stdout=f,stderr=subprocess.STDOUT)
record={'argv':argv,'exit':p.returncode,'seconds':time.monotonic()-start,'before':before,'after':hashes(),'log':str(r/(label+'.log'))}
(r/(label+'-command.json')).write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'label':label,'exit':p.returncode,'seconds':record['seconds'],'log':record['log']}));sys.exit(p.returncode)

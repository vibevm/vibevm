from pathlib import Path
import subprocess,json,hashlib
r=Path(__file__).parent
roots=['crates/vibe-wire/src/generated','crates/progress-core/src/generated','vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/generated','vibevm/vibepacks/org.vibevm.doc/web/v1.1.0/site/src/generated/doc-manifest.ts']
def hashes():
 out={}
 for name in roots:
  p=Path(name)
  for f in ([p] if p.is_file() else p.rglob('*')):
   if f.is_file(): out[str(f)]=hashlib.sha256(f.read_bytes()).hexdigest()
 return out
results=[]
for verb in ['check-codegen','wire-diff']:
 before=hashes()
 code=subprocess.run(['python3',str(r/'worker-run.py'),'worker-'+verb,'cargo','--offline','--locked','xtask',verb]).returncode
 after=hashes();results.append({'verb':verb,'exit':code,'generated_unchanged':before==after,'before':before,'after':after})
 (r/'worker-oracle-results.json').write_text(json.dumps(results,indent=2))
print(json.dumps([{k:v for k,v in x.items() if k not in ['before','after']} for x in results]))
